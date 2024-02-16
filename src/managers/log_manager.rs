use std::{borrow::BorrowMut, ops::Add, sync::Arc, time::Duration};

use poise::serenity_prelude::UserId;

use crate::{
    managers::{cotd_manager::SECONDS_IN_A_DAY, storage_manager::DataType},
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

impl LogManager {
    pub fn new(db: Arc<SurrealClient>, storage_manager: Arc<StorageManager>) -> Self {
        Self {
            db,
            storage_manager,
        }
    }
    async fn get_data_holder(&self, user_id: &UserId) -> Result<DataHolder, Error> {
        self.storage_manager
            .load_or(
                &format!("logs/{user_id}.txt"),
                true,
                DataType::String("".to_string()),
                Duration::from_secs(SECONDS_IN_A_DAY),
            )
            .await
    }

    fn get_path(user_id: &UserId) -> String {
        format!("logs/{user_id}.txt")
    }

    async fn save_data_holder(
        &self,
        user_id: &UserId,
        data_holder: &DataHolder,
    ) -> Result<(), Error> {
        self.storage_manager
            .save(
                &Self::get_path(user_id),
                data_holder,
                Duration::from_secs(SECONDS_IN_A_DAY),
            )
            .await?;

        Ok(())
    }

    pub async fn get_user_logs(&self, user_id: &UserId) -> Result<String, Error> {
        let data_holder = self.get_data_holder(user_id).await?;

        let read = data_holder.data.read().await;
        let string = read.string().cloned().unwrap_or_default();

        Ok(string)
    }

    pub async fn add_user_log(&self, user_id: &UserId, add_log: String) -> Result<(), Error> {
        let data_holder = self.get_data_holder(user_id).await?;

        let mut write = data_holder.data.write().await;
        let logs = write.string_mut().unwrap();

        //TODO: Change the prefix to be configurable.
        logs.insert_str(logs.len(), &format!("\n[MESSAGE:CUSTOM] {add_log}"));

        drop(write);

        //if error it will still save in ram so idc.
        let _ = self.save_data_holder(user_id, &data_holder).await;

        //TODO: make is to if the previous error is the same then it just adds a (x2) at the end to not clutter when database errors happen.
        /*let Some((_, last_error)) = logs.rsplit_once("\n") else {
            logs.insert_str(logs.len(), &add_log);
            return Ok(());
        };*/

        Ok(())
    }

    pub async fn clear_user_log(&self, user_id: &UserId) -> Result<(), Error> {
        self.storage_manager
            .delete(&Self::get_path(user_id))
            .await?;

        Ok(())
    }
}
