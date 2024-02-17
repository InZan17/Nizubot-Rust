use std::{borrow::BorrowMut, ops::Add, sync::Arc, time::Duration};

use poise::serenity_prelude::UserId;

use crate::{
    managers::{cotd_manager::SECONDS_IN_A_DAY, storage_manager::DataType},
    utils::IdType,
    Error,
};

use super::{
    db::SurrealClient,
    storage_manager::{DataHolder, StorageManager},
};

pub struct LogManager {
    db: Arc<SurrealClient>,
    storage_manager: Arc<StorageManager>,
}

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
pub enum LogSource {
    Server,
    User,
    Custom(String),
}

impl LogSource {
    pub fn to_string(&self) -> String {
        match self {
            LogSource::Server => "SERVER".to_string(),
            LogSource::User => "USER".to_string(),
            LogSource::Custom(string) => string.to_owned(),
        }
    }
}

impl LogManager {
    pub fn new(db: Arc<SurrealClient>, storage_manager: Arc<StorageManager>) -> Self {
        Self {
            db,
            storage_manager,
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

        //TODO: Change the prefix to be configurable.
        logs.insert_str(
            logs.len(),
            &format!(
                "\n[{}:{}] {add_log}",
                log_type.to_string(),
                log_source.to_string()
            ),
        );

        drop(write);

        //if error it will still save in ram so idc.
        let _ = self.save_data_holder(id, &data_holder).await;

        //TODO: make is to if the previous error is the same then it just adds a (x2) at the end to not clutter when database errors happen.
        /*let Some((_, last_error)) = logs.rsplit_once("\n") else {
            logs.insert_str(logs.len(), &add_log);
            return Ok(());
        };*/

        Ok(())
    }

    pub async fn clear_log(&self, id: &IdType) -> Result<(), Error> {
        self.storage_manager.delete(&Self::get_path(id)).await?;

        Ok(())
    }
}
