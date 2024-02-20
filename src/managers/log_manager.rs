use std::{
    borrow::BorrowMut,
    ops::Add,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use poise::serenity_prelude::{self, UserId};

use crate::{
    managers::{cotd_manager::SECONDS_IN_A_DAY, storage_manager::DataType},
    utils::IdType,
    Error,
};

use super::{
    db::SurrealClient,
    storage_manager::{DataHolder, StorageManager},
};

//TODO: Make log manager use its own solution to writing files.
pub struct LogManager {
    db: Arc<SurrealClient>,
    storage_manager: Arc<StorageManager>,
    log_path: PathBuf,
    owner_user_ids: Vec<UserId>,
    owner_webhook: Option<String>,
}

#[derive(Clone, Copy)]
pub enum LogType {
    Message,
    Warning,
    Error,
}

impl LogType {
    pub fn to_string(&self) -> String {
        match self {
            LogType::Message => "MESSAGE".to_string(),
            LogType::Warning => "WARNING".to_string(),
            LogType::Error => "ERROR".to_string(),
        }
    }
}

#[derive(Clone)]
pub enum LogSource {
    Guild,
    User,
    MessageDetector,
    ReactionRole,
    CotdRole,
    Reminder,
    Custom(String),
}

impl LogSource {
    pub fn to_string(&self) -> String {
        match self {
            LogSource::Guild => "GUILD".to_string(),
            LogSource::User => "USER".to_string(),
            LogSource::MessageDetector => "MESSAGE_DETECTOR".to_string(),
            LogSource::ReactionRole => "REACTION_ROLE".to_string(),
            LogSource::CotdRole => "COTD_ROLE".to_string(),
            LogSource::Reminder => "REMINDER".to_string(),
            LogSource::Custom(string) => string.to_owned(),
        }
    }
}

impl LogManager {
    pub fn new(
        db: Arc<SurrealClient>,
        storage_manager: Arc<StorageManager>,
        log_path: PathBuf,
        owner_user_ids: Vec<UserId>,
        owner_webhook: Option<String>,
    ) -> Self {
        Self {
            db,
            storage_manager,
            log_path,
            owner_user_ids,
            owner_webhook,
        }
    }
    async fn get_data_holder(&self, id: &IdType) -> Result<DataHolder, Error> {
        self.storage_manager
            .load_or(
                &Self::get_path(id),
                true,
                DataType::String("".to_string()),
                Duration::from_secs(SECONDS_IN_A_DAY),
            )
            .await
    }

    fn get_path(id: &IdType) -> String {
        match id {
            IdType::UserId(user_id) => format!("logs/user_{user_id}.txt"),
            IdType::GuildId(guild_id) => format!("logs/guild_{guild_id}.txt"),
        }
    }

    async fn save_data_holder(&self, id: &IdType, data_holder: &DataHolder) -> Result<(), Error> {
        self.storage_manager
            .save(
                &Self::get_path(id),
                data_holder,
                Duration::from_secs(SECONDS_IN_A_DAY),
            )
            .await?;

        Ok(())
    }

    pub async fn get_logs(&self, id: &IdType) -> Result<String, Error> {
        let data_holder = self.get_data_holder(id).await?;

        let read = data_holder.data.read().await;
        let string = read.string().cloned().unwrap_or_default();

        Ok(string)
    }

    pub async fn add_owner_log(
        &self,
        add_log: String,
        log_type: LogType,
        log_source: LogSource,
    ) -> Result<(), Error> {
        let mut result = Ok(());
        for owner_id in self.owner_user_ids.iter() {
            let res = self
                .add_log(
                    &IdType::UserId(*owner_id),
                    add_log.clone(),
                    log_type,
                    log_source.clone(),
                )
                .await;

            if res.is_err() {
                result = res;
            }
        }

        //TODO: use webhook too
        result
    }

    pub async fn add_log(
        &self,
        id: &IdType,
        add_log: String,
        log_type: LogType,
        log_source: LogSource,
    ) -> Result<(), Error> {
        let data_holder = self.get_data_holder(id).await?;

        let mut write = data_holder.data.write().await;
        let logs = write.string_mut().unwrap();

        let add_str = format!(
            "[{}:{}] {add_log}",
            log_type.to_string(),
            log_source.to_string()
        );

        // I'm sorry for the code
        if let Some((rest, last_error)) = logs.rsplit_once("\n") {
            //Check if previous error is the same.
            if add_str == last_error {
                //Same error. Adding (x2) to save space.
                logs.insert_str(logs.len(), " (x2)");
            } else if let Some((error, number)) = last_error.rsplit_once(" ") {
                // Check if removing the (xn) makes it match.
                if add_str == error {
                    //Check if it ends with (xn) where n is a number
                    if let Some(number) = extract_number(number) {
                        *logs = format!("{rest}\n{add_str} (x{})", number + 1);
                    } else {
                        logs.insert_str(logs.len(), &format!("\n{add_str}"));
                    }
                } else {
                    logs.insert_str(logs.len(), &format!("\n{add_str}"));
                }
            } else {
                logs.insert_str(logs.len(), &format!("\n{add_str}"));
            }
        } else {
            logs.insert_str(logs.len(), &format!("\n{add_str}"));
        };

        drop(write);

        //if error it will still save in ram so idc.
        let _ = self.save_data_holder(id, &data_holder).await;

        Ok(())
    }

    pub async fn clear_log(&self, id: &IdType) -> Result<(), Error> {
        self.storage_manager.delete(&Self::get_path(id)).await?;

        Ok(())
    }
}

fn extract_number(s: &str) -> Option<u64> {
    if s.starts_with("(x") && s.ends_with(")") && s.len() > 3 {
        let only_number = &s[2..(s.len() - 1)];
        return only_number.parse::<u64>().ok();
    }
    None
}
