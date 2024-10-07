use std::{sync::Arc, vec};

use poise::serenity_prelude::{self, CreateMessage, Message};
use serde::{Deserialize, Serialize};

use crate::{utils::IdType, Error};

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

pub struct DetectorManager {
    pub db: Arc<SurrealClient>,
}

impl DetectorManager {
    pub fn new(db: Arc<SurrealClient>) -> Self {
        Self { db }
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
        let db = &self.db;

        let detectors_option = match db.get_all_message_detectors(&id).await {
            Ok(ok) => ok,
            Err(err) => {
                return Err(DetectorError::Database(
                    err,
                    "Couldn't fetch detectors from guild.".to_string(),
                ))
            }
        };

        if let Some(detectors) = detectors_option {
            if detectors.len() >= 10 {
                return Err(DetectorError::Max10Detectors);
            }
        }

        let detector_info = DetectorInfo {
            detect_type,
            key,
            response,
            case_sensitive,
        };

        if let Err(err) = db.add_message_detector(&id, &detector_info).await {
            return Err(DetectorError::Database(
                err,
                "Couldn't add detector to guild.".to_string(),
            ));
        }

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
        let db = &self.db;

        if let Err(err) = db.remove_message_detector(&id, index).await {
            return Err(DetectorError::Database(
                err,
                "Couldn't remove detector from guild.".to_string(),
            ));
        }

        return Ok(());
    }

    /// Gets all detectors from a guild / user dm.
    ///
    /// Will error if database isn't connected or communication doesn't work.
    /// May also error if unable to parse response or if database returns an error.
    pub async fn get_message_detects(
        &self,
        id: IdType,
    ) -> Result<Vec<DetectorInfo>, DetectorError> {
        let db = &self.db;

        match db.get_all_message_detectors(&id).await {
            Ok(detectors_option) => return Ok(detectors_option.unwrap_or(vec![])),
            Err(err) => {
                return Err(DetectorError::Database(
                    err,
                    "Failed to get message detectors.".to_string(),
                ))
            }
        };
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

        let db = &self.db;

        let id;

        if let Some(guild_id) = message.guild_id {
            id = IdType::GuildId(guild_id);
        } else {
            id = IdType::UserId(message.author.id);
        }

        let detectors_option = match db.get_all_message_detectors(&id).await {
            Ok(ok) => ok,
            Err(err) => {
                return Err(DetectorError::Database(
                    err,
                    "Failed to get message detectors.".to_string(),
                ))
            }
        };

        let Some(detectors) = detectors_option else {
            return Ok(());
        };

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

            let res = message
                .channel_id
                .send_message(ctx, CreateMessage::new().content(&detector_info.response))
                .await;

            if let Err(err) = res {
                return Err(DetectorError::Serenity(
                    err,
                    "Couldn't send detector response.".to_string(),
                ));
            }
            break;
        }

        return Ok(());
    }
}
