use std::{
    collections::HashSet,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use poise::serenity_prelude::{ChannelId, GuildId, UserId};
use serde::{Deserialize, Serialize};

use super::storage_manager::{StorageManager, DataHolder};

pub struct RemindManager {
    storage_manager: Arc<StorageManager>,
    wait_until: u64
}

#[derive(Serialize, Deserialize)]
pub struct RemindInfo {
    original_time: u64,
    request_time: u64,
    finish_time: u64,
    channel_id: Option<u64>,
    guild_id: Option<u64>,
    user_id: u64,
    pub message_id: Option<u64>,
    message: Option<String>,
    looping: bool,
}

impl RemindManager {
    pub fn new(storage_manager: Arc<StorageManager>) -> Self {
        RemindManager { storage_manager, wait_until: 0 }
    }

    pub async fn add_reminder(
        &self,
        guild_id: Option<u64>,
        channel_id: Option<u64>,
        user_id: UserId,
        duration: u64,
        looping: bool,
        message: Option<String>,
    ) -> Result<(usize, Arc<DataHolder<Vec<RemindInfo>>>), String> {
        if guild_id.is_some() != channel_id.is_some() {
            panic!("guild id and channel id should either both be there or both not be there.")
        }

        let reminding_users_data = self
            .storage_manager
            .get_data_or_default::<HashSet<u64>>(vec!["reminders"], HashSet::new())
            .await;
        let mut reminding_users_mut = reminding_users_data.get_data_mut().await;
        reminding_users_mut.insert(*user_id.as_u64());
        reminding_users_data.request_file_write().await;

        let user_reminders_data = self
            .storage_manager
            .get_data_or_default::<Vec<RemindInfo>>(
                vec!["users", &user_id.as_u64().to_string(), "reminders"],
                vec![],
            )
            .await;

        let mut user_reminders_mut = user_reminders_data.get_data_mut().await;

        if user_reminders_mut.len() >= 50 {
            return Err("You already have 50 different reminders elsewhere.".to_string());
        }

        let mut counter = 0;

        if let Some(guild_id) = &guild_id {
            for reminder in user_reminders_mut.iter() {
                if let Some(reminder_guild_id) = &reminder.guild_id {
                    if guild_id == reminder_guild_id {
                        counter += 1
                    }
                }
            }
        }
        // in dms we dont really care how many reminders they have.

        if counter >= 10 {
            return Err("You already have 10 reminders in this guild.".to_string());
        }

        let current_time = get_seconds();
        let finish_time = current_time + duration;

        let remind_info = RemindInfo {
            original_time: current_time,
            request_time: current_time,
            finish_time,
            channel_id,
            guild_id,
            user_id: *user_id.as_u64(),
            message_id: None,
            message,
            looping,
        };

        let mut index = 0;

        for i in 0..(user_reminders_mut.len() + 1) {

            index = i;

            if i == user_reminders_mut.len() {
                user_reminders_mut.push(remind_info);
                break;
            }

            let check_reminder = &user_reminders_mut[i];

            if check_reminder.finish_time > finish_time {
                user_reminders_mut.insert(i, remind_info);
                break;
            }
        }

        user_reminders_data.request_file_write().await;

        drop(user_reminders_mut);

        return Ok((index, user_reminders_data));
    }

    pub fn remove_reminder(&self) {}

    pub fn list_reminders(&self) {}
}

fn get_seconds() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards. Oopsie.");

    since_the_epoch.as_secs()
}
