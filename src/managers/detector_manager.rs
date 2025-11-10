use std::{sync::Arc, time::Duration, vec};

use poise::serenity_prelude::{self, CreateMessage, Message};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};

use crate::{
    utils::{IdType, TtlMap},
    Error,
};

use super::db::SurrealClient;

#[derive(Serialize, Deserialize, Clone, poise::ChoiceParameter)]
pub enum DetectType {
    #[name = "Starts with"]
    StartsWith,
    #[name = "Contains"]
    Contains,
    #[name = "Ends with"]
    EndsWith,
    #[name = "Equals"]
    Equals,
}

impl DetectType {
    /// Returns a string that can be used in a sentence.
    pub fn to_sentence(&self) -> &str {
        match self {
            DetectType::StartsWith => "starts with",
            DetectType::Contains => "contains",
            DetectType::EndsWith => "ends with",
            DetectType::Equals => "equals",
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DetectorInfo {
    /// The string to detect.
    pub key: String,
    /// Response to when it detects the key.
    pub response: String,
    /// How to detect the key.
    #[serde(alias = "detectionType", alias = "detect_type")]
    pub detect_type: DetectType,
    /// If the detector should be case sensitive.
    #[serde(alias = "caseSensitive", alias = "case_sensitive")]
    pub case_sensitive: bool,
}

pub enum DetectorError {
    Max10Detectors,
    InvalidIndex,
    Database(Error, String),
    Serenity(serenity_prelude::Error, String),
}

impl DetectorError {
    pub fn to_string(&self) -> String {
        match self {
            DetectorError::Max10Detectors => {
                "You can only have a max amount of 10 message detectors.".to_string()
            }
            DetectorError::InvalidIndex => "Index isn't valid.".to_string(),
            DetectorError::Database(err, description) => format!("{description} {err}"),

            DetectorError::Serenity(err, description) => format!("{description} {err}"),
        }
    }
}

pub struct DetectorsData {
    id: IdType,
    detectors: Option<Vec<DetectorInfo>>,
}

impl DetectorsData {
    pub fn new(id: IdType) -> Self {
        Self {
            id,
            detectors: None,
        }
    }

    pub async fn get_detectors(
        &mut self,
        db: &SurrealClient,
    ) -> Result<&mut Vec<DetectorInfo>, Error> {
        let detectors_mut = &mut self.detectors;
        match detectors_mut {
            Some(detectors) => return Ok(detectors),
            None => {
                let fetched_detectors = db.get_all_message_detectors(self.id).await?;

                *detectors_mut = Some(fetched_detectors);
                return Ok(detectors_mut.as_mut().unwrap());
            }
        }
    }

    pub async fn add_detector(
        &mut self,
        detect_info: DetectorInfo,
        db: &SurrealClient,
    ) -> Result<(), Error> {
        let id = self.id;
        let detectors = self.get_detectors(db).await?;
        db.add_message_detector(id, &detect_info).await?;
        detectors.push(detect_info);
        Ok(())
    }

    pub async fn delete_detector(&mut self, index: usize, db: &SurrealClient) -> Result<(), Error> {
        let id = self.id;
        let detectors = self.get_detectors(db).await?;
        db.remove_message_detector(id, index).await?;
        detectors.remove(index);
        Ok(())
    }
}

pub struct DetectorManager {
    pub db: Arc<SurrealClient>,
    /// Holds detectors for different guilds/users.
    ///
    /// DetectorsData is inside of an Arc so that the RwLock gets locked as little as possible.
    /// This is also fine because DetectorsData uses interior mutability.
    ///
    /// As long as the Arc doesn't get saved anywhere / anything uses it for longer than the duration of the TtlMap,
    /// everything will be fine. The concern otherwise would be that the entry gets removed,
    /// and something still has an Arc from that entry and end up doing things that wont be properly saved.
    detectors_data: RwLock<TtlMap<IdType, Arc<Mutex<DetectorsData>>>>,
}

impl DetectorManager {
    pub fn new(db: Arc<SurrealClient>) -> Self {
        Self {
            db,
            detectors_data: RwLock::new(TtlMap::new(Duration::from_secs(60 * 60))),
        }
    }

    /// NOTE: It is VERY IMPORTANT that you do not store this Arc anywhere for long term use!
    pub async fn get_detectors_data(&self, id: IdType) -> Arc<Mutex<DetectorsData>> {
        if let Some(detectors_data) = self.detectors_data.read().await.get(&id).cloned() {
            return detectors_data;
        }

        let mut detectors_data_mut = self.detectors_data.write().await;
        if let Some(detectors_data) = detectors_data_mut.get(&id).cloned() {
            return detectors_data;
        }

        let detectors_data = Arc::new(Mutex::new(DetectorsData::new(id)));

        detectors_data_mut.insert(id, detectors_data.clone());

        detectors_data
    }

    /// Adds a detector to a guild / user dm.
    ///
    /// Will error if database isn't connected or communication doesn't work.
    /// May also error if unable to parse response or if database returns an error.
    pub async fn add_message_detect(
        &self,
        detect_type: DetectType,
        key: String,
        response: String,
        case_sensitive: bool,
        id: IdType,
    ) -> Result<(), DetectorError> {
        let detectors_data = self.get_detectors_data(id).await;
        let mut locked_detectors_data = detectors_data.lock().await;
        let db = &self.db;

        let detectors = locked_detectors_data
            .get_detectors(db)
            .await
            .map_err(|err| {
                DetectorError::Database(err, "Couldn't fetch detectors from guild.".to_string())
            })?;

        if detectors.len() >= 10 {
            return Err(DetectorError::Max10Detectors);
        }

        let detector_info = DetectorInfo {
            detect_type,
            key,
            response,
            case_sensitive,
        };

        locked_detectors_data
            .add_detector(detector_info, db)
            .await
            .map_err(|err| {
                DetectorError::Database(err, "Couldn't add detector to guild.".to_string())
            })?;

        Ok(())
    }

    /// Removes a detector to a guild / user dm.
    ///
    /// Will error if database isn't connected or communication doesn't work.
    /// May also error if unable to parse response or if database returns an error.
    pub async fn remove_message_detect(
        &self,
        index: usize,
        id: IdType,
    ) -> Result<(), DetectorError> {
        let detectors_data = self.get_detectors_data(id).await;
        let mut locked_detectors_data = detectors_data.lock().await;
        let db = &self.db;

        let detectors = locked_detectors_data
            .get_detectors(db)
            .await
            .map_err(|err| {
                DetectorError::Database(err, "Couldn't fetch detectors from guild.".to_string())
            })?;

        if index >= detectors.len() {
            return Err(DetectorError::InvalidIndex);
        }

        locked_detectors_data
            .delete_detector(index, db)
            .await
            .map_err(|err| {
                DetectorError::Database(err, "Couldn't delete detector from guild.".to_string())
            })?;

        Ok(())
    }

    /// Responds to message if it matches a detector.
    /// Will not do anything if the message author is a bot.
    ///
    /// Will error if database isn't connected or communication doesn't work.
    /// May also error if unable to parse response or if database returns an error.
    /// Will also error if sending the message to the channel doesn't work.
    pub async fn on_message(
        &self,
        ctx: &serenity_prelude::Context,
        message: &Message,
    ) -> Result<(), DetectorError> {
        if message.author.bot {
            return Ok(());
        }

        let id;

        if let Some(guild_id) = message.guild_id {
            id = IdType::GuildId(guild_id);
        } else {
            id = IdType::UserId(message.author.id);
        }

        let detectors_data = self.get_detectors_data(id).await;
        let mut locked_detectors_data = detectors_data.lock().await;
        let db = &self.db;

        let detectors = locked_detectors_data
            .get_detectors(db)
            .await
            .map_err(|err| {
                DetectorError::Database(err, "Failed to get message detectors.".to_string())
            })?;

        for detector_info in detectors.iter() {
            let key = {
                if detector_info.case_sensitive {
                    detector_info.key.clone()
                } else {
                    detector_info.key.to_ascii_lowercase()
                }
            };

            let content = {
                if detector_info.case_sensitive {
                    message.content.clone()
                } else {
                    message.content.to_ascii_lowercase()
                }
            };

            let should_send = {
                match &detector_info.detect_type {
                    DetectType::StartsWith => content.starts_with(&key),
                    DetectType::Contains => content.contains(&key),
                    DetectType::EndsWith => content.ends_with(&key),
                    DetectType::Equals => content == key,
                }
            };

            if !should_send {
                continue;
            }

            let create_message = CreateMessage::new().content(&detector_info.response);

            drop(locked_detectors_data);

            message
                .channel_id
                .send_message(ctx, create_message)
                .await
                .map_err(|err| {
                    DetectorError::Serenity(err, "Couldn't send detector response.".to_string())
                })?;

            break;
        }

        return Ok(());
    }
}

pub fn detector_manager_loop(detector_manager: Arc<DetectorManager>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30 * 60)).await;
            let mut guild_data_write = detector_manager.detectors_data.write().await;
            guild_data_write.clear_expired();
        }
    });
}
