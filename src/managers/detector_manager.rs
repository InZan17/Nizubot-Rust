use core::borrow;
use std::{sync::Arc, vec};

use poise::serenity_prelude::{self, Message, MessageAction};
use serde::{Deserialize, Serialize};

use crate::{Context, Error};

use super::storage_manager::{self, StorageManager};

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
    #[serde(alias = "detectionType", alias = "detect_type")]
    pub detect_type: DetectType,
    pub key: String,
    pub response: String,
    #[serde(alias = "caseSensitive", alias = "case_sensitive")]
    pub case_sensitive: bool,
}

pub struct DetectorManager {
    pub storage_manager: Arc<StorageManager>,
}

impl DetectorManager {
    pub fn new(storage_manager: Arc<StorageManager>) -> Self {
        Self { storage_manager }
    }

    pub async fn add_message_detect(
        &self,
        detect_type: DetectType,
        key: String,
        response: String,
        case_sensitive: bool,
        guild_or_user_id: u64,
        is_dms: bool,
    ) -> Result<(), String> {
        let storage_manager = &self.storage_manager;

        let id_as_string = guild_or_user_id.to_string();

        let detectors;

        if is_dms {
            detectors = storage_manager
                .get_data_or_default::<Vec<DetectorInfo>>(
                    vec!["users", &id_as_string, "message_detectors"],
                    vec![],
                )
                .await;
        } else {
            detectors = storage_manager
                .get_data_or_default::<Vec<DetectorInfo>>(
                    vec!["guilds", &id_as_string, "message_detectors"],
                    vec![],
                )
                .await;
        }

        let mut detectors_mut = detectors.get_data_mut().await;

        if detectors_mut.len() >= 10 {
            return Err("You can only have a max amount of 10 message detectors.".to_string());
        }

        let detector_info = DetectorInfo {
            detect_type,
            key,
            response,
            case_sensitive,
        };

        detectors_mut.push(detector_info);

        detectors.request_file_write().await;

        return Ok(());
    }

    pub async fn remove_message_detect(
        &self,
        index: usize,
        guild_or_user_id: u64,
        is_dms: bool,
    ) -> Result<(), String> {
        let storage_manager = &self.storage_manager;

        let id_as_string = guild_or_user_id.to_string();

        let detectors;

        if is_dms {
            detectors = storage_manager
                .get_data_or_default::<Vec<DetectorInfo>>(
                    vec!["users", &id_as_string, "message_detectors"],
                    vec![],
                )
                .await;
        } else {
            detectors = storage_manager
                .get_data_or_default::<Vec<DetectorInfo>>(
                    vec!["guilds", &id_as_string, "message_detectors"],
                    vec![],
                )
                .await;
        }

        let mut detectors_mut = detectors.get_data_mut().await;

        if detectors_mut.len() <= index {
            return Err("Index isn't valid.".to_string());
        }

        detectors_mut.remove(index);

        detectors.request_file_write().await;

        return Ok(());
    }

    pub async fn get_message_detects(
        &self,
        guild_or_user_id: u64,
        is_dms: bool,
    ) -> Vec<DetectorInfo> {
        let storage_manager = &self.storage_manager;

        let id_as_string = guild_or_user_id.to_string();

        let detectors;

        if is_dms {
            detectors = storage_manager
                .get_data_or_default::<Vec<DetectorInfo>>(
                    vec!["users", &id_as_string, "message_detectors"],
                    vec![],
                )
                .await;
        } else {
            detectors = storage_manager
                .get_data_or_default::<Vec<DetectorInfo>>(
                    vec!["guilds", &id_as_string, "message_detectors"],
                    vec![],
                )
                .await;
        }

        let detectors_read = detectors.get_data().await;

        return detectors_read.clone();
    }

    pub async fn on_message(
        &self,
        ctx: &serenity_prelude::Context,
        message: &Message,
    ) -> Result<(), Error> {
        if message.author.bot {
            return Ok(());
        }

        let storage_manager = &self.storage_manager;

        let detectors;

        if let Some(guild_id) = message.guild_id {
            let id_as_string = guild_id.to_string();
            detectors = storage_manager
                .get_data_or_default::<Vec<DetectorInfo>>(
                    vec!["guilds", &id_as_string, "message_detectors"],
                    vec![],
                )
                .await;
        } else {
            let id_as_string = message.author.id.to_string();
            detectors = storage_manager
                .get_data_or_default::<Vec<DetectorInfo>>(
                    vec!["users", &id_as_string, "message_detectors"],
                    vec![],
                )
                .await;
        }

        let detectors_read = detectors.get_data().await;

        for detector_info in detectors_read.iter() {
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

            if should_send {
                message.channel_id
                    .send_message(ctx, |m| m.content(&detector_info.response))
                    .await?;
                break;
            }
        }

        return Ok(());
    }
}
