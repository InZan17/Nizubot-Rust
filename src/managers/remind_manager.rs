use std::{
    collections::HashSet,
    future::Future,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use poise::serenity_prelude::{
    CacheHttp, Channel, ChannelId, Context, CreateMessage, GuildId, Message, MessageId,
    MessageReference, UserId,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::Error;

use super::storage_manager::{self, DataHolder, StorageManager};

pub struct RemindManager {
    storage_manager: Arc<StorageManager>,
    pub wait_until: Mutex<u64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RemindInfo {
    pub original_time: u64,
    pub request_time: u64,
    pub finish_time: u64,
    pub channel_id: Option<u64>,
    pub guild_id: Option<u64>,
    pub user_id: u64,
    pub message_id: Option<u64>,
    pub message: Option<String>,
    pub looping: bool,
}

impl RemindManager {
    pub fn new(storage_manager: Arc<StorageManager>) -> Self {
        RemindManager {
            storage_manager,
            wait_until: Mutex::new(0),
        }
    }

    pub async fn add_reminder<'a, F, Fut>(
        &self,
        guild_id: Option<u64>,
        channel_id: Option<u64>,
        user_id: UserId,
        duration: u64,
        looping: bool,
        message: Option<String>,
        message_id_callback: F,
    ) -> Result<(), Error>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<u64, Error>>,
    {
        if guild_id.is_some() != channel_id.is_some() {
            panic!("guild_id and channel_id should either both be Some or both be None.")
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
            return Err(Error::from("You already have 10 reminders in this guild."));
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
            return Err(Error::from("You already have 10 reminders in this guild."));
        }

        let current_time = get_seconds();
        let finish_time = current_time + duration;

        let mut remind_info = RemindInfo {
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

        let message_id = message_id_callback().await?;

        remind_info.message_id = Some(message_id);

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

        let mut mut_wait_until = self.wait_until.lock().await;
        *mut_wait_until = mut_wait_until.min(finish_time);
        drop(mut_wait_until);

        return Ok(());
    }

    pub async fn remove_reminder(
        &self,
        user_id: u64,
        guild_id: Option<u64>,
        removal_index: usize,
    ) -> Option<RemindInfo> {
        let user_reminders_data = self
            .storage_manager
            .get_data_or_default::<Vec<RemindInfo>>(
                vec!["users", &user_id.to_string(), "reminders"],
                vec![],
            )
            .await;

        let mut user_reminders_mut = user_reminders_data.get_data_mut().await;

        let mut reminders_index = 0;
        let mut reminders_guild_index = 0;
        let mut found = false;

        for (index, reminder) in user_reminders_mut.iter().enumerate() {
            reminders_index = index;
            if reminder.guild_id == guild_id {
                if reminders_guild_index == removal_index {
                    found = true;
                    break;
                }
                reminders_guild_index += 1;
            }
        }

        if !found {
            return None;
        }
        let removed_reminder = user_reminders_mut.remove(reminders_index);

        user_reminders_data.request_file_write().await;

        return Some(removed_reminder);
    }

    pub async fn list_reminders(&self, user_id: u64, guild_id: Option<u64>) -> Vec<RemindInfo> {
        let user_reminders_data = self
            .storage_manager
            .get_data_or_default::<Vec<RemindInfo>>(
                vec!["users", &user_id.to_string(), "reminders"],
                vec![],
            )
            .await;

        let user_reminders = user_reminders_data.get_data().await;

        let mut reminders = vec![];

        for reminder in user_reminders.iter() {
            if reminder.guild_id == guild_id {
                reminders.push(reminder.clone());
            }
        }

        reminders
    }
}

fn get_seconds() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards. Oopsie.");

    since_the_epoch.as_secs()
}

pub fn remind_manager_loop(arc_ctx: Arc<Context>, remind_manager: Arc<RemindManager>) {
    tokio::spawn(async move {
        let storage_manager = &remind_manager.storage_manager;

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let current_time = get_seconds();

            let wait_until = remind_manager.wait_until.lock().await.clone();

            // TODO: ping discord. If it doesnt work then dont try to remind people.

            if wait_until > current_time {
                continue;
            }

            let mut next_wait_until = u64::MAX;

            let reminders_data = storage_manager
                .get_data_or_default::<HashSet<u64>>(vec!["reminders"], HashSet::new())
                .await;

            let mut set_of_reminding_users = reminders_data.get_data_mut().await;

            let mut remove_users = vec![1];

            for user_id in set_of_reminding_users.iter() {
                let user_reminders_data = storage_manager
                    .get_data_or_default::<Vec<RemindInfo>>(
                        vec!["users", &user_id.to_string(), "reminders"],
                        vec![],
                    )
                    .await;

                let mut user_reminders_data_mut = user_reminders_data.get_data_mut().await;

                if user_reminders_data_mut.len() == 0 {
                    remove_users.push(*user_id);
                }

                let mut add_reminders = vec![];
                let mut remove_reminders = vec![];
                for (index, reminder_info) in user_reminders_data_mut.iter().enumerate() {
                    if reminder_info.finish_time > current_time {
                        next_wait_until = next_wait_until.min(reminder_info.finish_time);
                        break;
                    }

                    let channel = get_channel(reminder_info, arc_ctx.clone()).await;

                    let Some(channel) = channel else {
                        continue;
                    };

                    let message_ending;

                    if let Some(message) = &reminder_info.message {
                        message_ending = format!(" with: {message}")
                    } else {
                        message_ending = ".".to_string()
                    }

                    let time_difference = current_time - reminder_info.finish_time;

                    let message_refrence_opt;

                    if let Some(message_id) = reminder_info.message_id {
                        let mut message_refrence =
                            MessageReference::from((channel.id(), MessageId(message_id)));
                        if let Some(guild_id) = reminder_info.guild_id {
                            message_refrence.guild_id = Some(GuildId(guild_id));
                        }
                        message_refrence_opt = Some(message_refrence);
                    } else {
                        message_refrence_opt = None;
                    }

                    println!("{}", message_refrence_opt.is_some());

                    if reminder_info.looping {
                        let wait_time = reminder_info.finish_time - reminder_info.request_time;
                        let missed_reminders =
                            (current_time - reminder_info.request_time) / wait_time - 1;

                        // TODO: if res is None or Err, add it to remove_reminders.
                        if time_difference > 60 {
                            let res = send_message(&channel, arc_ctx.clone(), |m| {
                                if let Some(message_refrence) = message_refrence_opt {
                                    m.reference_message(message_refrence);
                                }
                                m.allowed_mentions(|a| {a.users(vec![reminder_info.user_id])}).content(format!("Sorry <@!{}>, I was supposed to remind you <t:{}:R>! <t:{}:R> you told me to keep reminding you{message_ending}", reminder_info.user_id, reminder_info.finish_time, reminder_info.original_time))}).await;
                        } else {
                            let res = send_message(&channel, arc_ctx.clone(), |m| {
                                if let Some(message_refrence) = message_refrence_opt {
                                    m.reference_message(message_refrence);
                                }
                                m.allowed_mentions(|a| {a.users(vec![reminder_info.user_id])}).content(format!("<@!{}>! <t:{}:R> you told me to keep reminding you{message_ending}", reminder_info.user_id, reminder_info.original_time))}).await;
                        }

                        let mut new_reminder = reminder_info.clone();

                        new_reminder.request_time =
                            new_reminder.finish_time + wait_time * missed_reminders;
                        new_reminder.finish_time = new_reminder.request_time + wait_time;

                        next_wait_until = next_wait_until.min(new_reminder.finish_time);

                        add_reminders.push(new_reminder);
                    } else {
                        if time_difference > 60 {
                            let res = send_message(&channel, arc_ctx.clone(), |m| {
                                if let Some(message_refrence) = message_refrence_opt {
                                    m.reference_message(message_refrence);
                                }
                                m.allowed_mentions(|a| {a.users(vec![reminder_info.user_id])}).content(format!("Sorry <@!{}>, I was supposed to remind you <t:{}:R>! <t:{}:R> you told me to remind you{message_ending}", reminder_info.user_id, reminder_info.finish_time, reminder_info.original_time))}).await;
                        } else {
                            let res = send_message(&channel, arc_ctx.clone(), |m| {
                                if let Some(message_refrence) = message_refrence_opt {
                                    m.reference_message(message_refrence);
                                }
                                m.allowed_mentions(|a| {a.users(vec![reminder_info.user_id])}).content(format!("<@!{}>! <t:{}:R> you told me to remind you{message_ending}", reminder_info.user_id, reminder_info.original_time))}).await;
                        }
                    }

                    remove_reminders.push(index);
                }

                // TODO: Make 2 remove reminders arrays. One for  those that we are certain shall be removed and one where we are a bit unsure. Maybe include a bool in a tuple.
                // All the certain reminders are removed immediately. Meanwhile the uncertain ones should only be removed if when pinging discord fails.
                for removal_index in remove_reminders.iter().rev() {
                    user_reminders_data_mut.remove(*removal_index);
                }

                for new_reminder in add_reminders {
                    for i in 0..user_reminders_data_mut.len() + 1 {
                        if i == user_reminders_data_mut.len() {
                            user_reminders_data_mut.push(new_reminder);
                            break;
                        }

                        let reminder = &user_reminders_data_mut[i];

                        if reminder.finish_time > new_reminder.finish_time {
                            user_reminders_data_mut.insert(i, new_reminder);
                            break;
                        }
                    }
                }

                if remove_reminders.len() != 0 {
                    user_reminders_data.request_file_write().await;
                }
            }

            for user_id in remove_users.iter() {
                set_of_reminding_users.remove(user_id);
            }

            if remove_users.len() != 0 {
                reminders_data.request_file_write().await;
            }

            *remind_manager.wait_until.lock().await = next_wait_until;
        }
    });
}

async fn send_message<'a, F>(
    channel: &Channel,
    arc_ctx: Arc<Context>,
    f: F,
) -> Option<poise::serenity_prelude::Result<Message>>
where
    for<'b> F: FnOnce(&'b mut CreateMessage<'a>) -> &'b mut CreateMessage<'a>,
{
    if let Some(channel) = &channel.clone().guild() {
        return Some(channel.send_message(arc_ctx, f).await);
    } else if let Some(channel) = &channel.clone().private() {
        return Some(channel.send_message(arc_ctx, f).await);
    } else {
        return None;
    }
}

async fn get_channel(reminder_info: &RemindInfo, arc_ctx: Arc<Context>) -> Option<Channel> {
    if let Some(guild_id) = reminder_info.guild_id {
        if let Some(channel_id) = reminder_info.channel_id {
            if let Some(guild) = arc_ctx.cache.guild(guild_id) {
                let channel_test = guild.channels.get(&ChannelId(channel_id)).cloned();
                return channel_test;
            } else {
                let guild_res = arc_ctx.http.get_guild(guild_id).await;
                if let Ok(guild) = guild_res {
                    let channel_res = guild.channels(arc_ctx).await;
                    if let Ok(ok_channel) = channel_res {
                        let guild_channel = ok_channel.get(&ChannelId(channel_id)).cloned();
                        if let Some(guild_channel) = guild_channel {
                            return Some(Channel::Guild(guild_channel));
                        } else {
                            return None;
                        }
                    }
                }
            }
        }
        return None;
    } else {
        let user;
        if let Some(some_user) = arc_ctx.cache.user(reminder_info.user_id) {
            user = some_user;
        } else if let Ok(ok_user) = arc_ctx.http.get_user(reminder_info.user_id).await {
            user = ok_user;
        } else {
            return None;
        }
        if let Ok(dm_channel) = user.create_dm_channel(&arc_ctx).await {
            return Some(Channel::Private(dm_channel));
        }
    }
    None
}
