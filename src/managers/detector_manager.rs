use core::borrow;
use std::{sync::Arc, vec};

use poise::serenity_prelude::{self, Message, MessageAction};
use serde::{Deserialize, Serialize};

use crate::{utils::IdType, Context, Error};

use super::{
    db::SurrealClient,
    storage_manager::{self, StorageManager},
};

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
    ) -> Result<(), Error> {
        let db = &self.db;

        let detectors_option = db.get_all_message_detectors(&id).await?;

        if let Some(detectors) = detectors_option {
            if detectors.len() >= 10 {
                return Err("You can only have a max amount of 10 message detectors.".into());
            }
        }

        let detector_info = DetectorInfo {
            detect_type,
            key,
            response,
            case_sensitive,
        };

        db.add_message_detector(&id, &detector_info).await?;

        Ok(())
    }

    /// Removes a detector to a guild / user dm.
    ///
    /// Will error if database isn't connected or communication doesn't work.
    /// May also error if unable to parse response or if database returns an error.
    pub async fn remove_message_detect(
        &self,
        index: usize,
        // TODO: Merge guild_or_user_id and is_dms into an enum.
        guild_or_user_id: u64,
        is_dms: bool,
    ) -> Result<(), Error> {
        let db = &self.db;

        let id_as_string = guild_or_user_id.to_string();

        let table_id;

        if is_dms {
            table_id = format!("user:{id_as_string}");
        } else {
            table_id = format!("guild:{id_as_string}");
        }

        let detectors_option: Option<Vec<DetectorInfo>> = db
            .query(format!(
                "SELECT VALUE message_detectors FROM {table_id} WHERE message_detectors"
            ))
            .await?
            .take(0)?;

        if let Some(detectors) = detectors_option {
            if detectors.len() <= index {
                return Err("Index isn't valid.".into());
            }
        } else {
            return Err("Index isn't valid.".into());
        }

        db.query(format!(
            "UPDATE {table_id} SET message_detectors = array::remove(message_detectors, {index});"
        ))
        .await?;

        return Ok(());
    }

    /// Gets all detectors from a guild / user dm.
    ///
    /// Will error if database isn't connected or communication doesn't work.
    /// May also error if unable to parse response or if database returns an error.
    pub async fn get_message_detects(&self, id: IdType) -> Result<Vec<DetectorInfo>, Error> {
        let db = &self.db;

        let detectors_option = db.get_all_message_detectors(&id).await?;

        return Ok(detectors_option.unwrap_or(vec![]));
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
    ) -> Result<(), Error> {
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

        let detectors_option = db.get_all_message_detectors(&id).await?;

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

            message
                .channel_id
                .send_message(ctx, |m| m.content(&detector_info.response))
                .await?;
            break;
        }

        return Ok(());
    }
}
