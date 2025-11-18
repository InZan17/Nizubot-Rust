use std::{future::Future, sync::Arc, time::Duration};

use poise::serenity_prelude::{
    ChannelId, Context, CreateAllowedMentions, CreateMessage, GuildId, MessageId, MessageReference,
    UserId,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};

use crate::{
    managers::log_manager::{LogSource, LogType},
    utils::{get_seconds, IdType, TtlMap},
    Error,
};

use super::{db::SurrealClient, log_manager::LogManager};

pub struct RemindersData {
    user_id: UserId,
    pub reminders: Option<Vec<RemindInfo>>,
}

impl RemindersData {
    pub fn new(user_id: UserId) -> Self {
        Self {
            user_id,
            reminders: None,
        }
    }

    pub async fn get_reminders(
        &mut self,
        db: &SurrealClient,
    ) -> Result<&mut Vec<RemindInfo>, Error> {
        let reminders_mut = &mut self.reminders;
        match reminders_mut {
            Some(reminders) => return Ok(reminders),
            None => {
                let fetched_reminders = db.list_user_reminders(self.user_id).await?;

                *reminders_mut = Some(fetched_reminders);
                return Ok(reminders_mut.as_mut().unwrap());
            }
        }
    }

    pub async fn add_reminder(
        &mut self,
        mut remind_info: RemindInfo,
        db: &SurrealClient,
    ) -> Result<(), Error> {
        let user_id = self.user_id;
        assert_eq!(user_id, remind_info.user_id);
        let reminders = self.get_reminders(db).await?;
        db.add_user_reminder(&mut remind_info).await?;
        reminders.push(remind_info);
        Ok(())
    }

    pub async fn delete_reminder(
        &mut self,
        removal_index: usize,
        guild_id: Option<GuildId>,
        db: &SurrealClient,
    ) -> Result<RemindInfo, Error> {
        let reminders = self.get_reminders(db).await?;

        let mut reminders_index = 0;
        let mut reminders_guild_index = 0;
        let mut found = false;

        for (current_index, reminder) in reminders.iter().enumerate() {
            reminders_index = current_index;
            if reminder.guild_id == guild_id {
                if reminders_guild_index == removal_index {
                    found = true;
                    break;
                }
                reminders_guild_index += 1;
            }
        }

        if !found {
            return Err("Failed to remove reminder. Are you using a valid index?".into());
        }

        let removing_reminder = &reminders[reminders_index];

        let Some(reminder_id) = &removing_reminder.id else {
            return Err("Reminder is missing a database id.".into());
        };

        db.delete_table_id(reminder_id).await?;

        Ok(reminders.remove(removal_index))
    }
}

pub struct RemindManager {
    db: Arc<SurrealClient>,
    pub reminders_data: RwLock<TtlMap<UserId, Arc<Mutex<RemindersData>>>>,
    pub wait_until: Mutex<u64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RemindInfo {
    pub original_time: u64,
    pub request_time: u64,
    pub finish_time: u64,
    pub channel_id: ChannelId,
    pub guild_id: Option<GuildId>,
    pub user_id: UserId,
    /// id will be Some() when retrieved from surrealdb. Otherwise None.
    #[serde(skip_serializing)]
    pub id: Option<String>,
    pub message_id: Option<MessageId>,
    pub message: Option<String>,
    pub looping: bool,
}

impl RemindManager {
    pub fn new(db: Arc<SurrealClient>) -> Self {
        RemindManager {
            db,
            reminders_data: RwLock::new(TtlMap::new(Duration::from_secs(60 * 60))),
            wait_until: Mutex::new(0),
        }
    }

    /// NOTE: It is VERY IMPORTANT that you do not store this Arc anywhere for long term use!
    pub async fn get_reminders_data(&self, user_id: UserId) -> Arc<Mutex<RemindersData>> {
        if let Some(reminders_data) = self.reminders_data.read().await.get(&user_id).cloned() {
            return reminders_data;
        }

        let mut reminders_data_mut = self.reminders_data.write().await;
        if let Some(reminders_data) = reminders_data_mut.get(&user_id).cloned() {
            return reminders_data;
        }

        let reminders_data = Arc::new(Mutex::new(RemindersData::new(user_id)));

        reminders_data_mut.insert(user_id, reminders_data.clone());

        reminders_data
    }

    /// Adds reminder
    ///
    /// guild_id is an Option because if the reminder is in dms then guild_id isn't required.
    ///
    /// Will error if unable to communicate with db or if callback errors.
    /// Will also error if there's too many reminders.
    ///
    /// Max total reminders = 50
    ///
    /// Max reminders per guild = 10
    pub async fn add_reminder<F, Fut>(
        &self,
        guild_id: Option<GuildId>,
        channel_id: ChannelId,
        user_id: UserId,
        duration: u64,
        looping: bool,
        message: Option<String>,
        // Callback that returns the message id of the message bot should reply to.
        message_id_callback: F,
    ) -> Result<(), Error>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<MessageId, Error>>,
    {
        let db = &self.db;

        let reminders_data = self.get_reminders_data(user_id).await;
        let mut locked_reminders_data = reminders_data.lock().await;

        let user_reminders = locked_reminders_data.get_reminders(&db).await?;

        if user_reminders.len() >= 50 {
            return Err(Error::from(
                "You've already got a total of 50 reminders, including other guilds. Consider removing some, or perhaps consider using a calendar, or some reminder app.",
            ));
        }

        let mut counter = 0;

        if let Some(guild_id) = &guild_id {
            for reminder in user_reminders.iter() {
                if let Some(reminder_guild_id) = &reminder.guild_id {
                    if guild_id == reminder_guild_id {
                        counter += 1
                    }
                }
            }
        }
        // in dms we dont really care how many reminders they have since it doesnt affect others.

        if counter >= 10 {
            return Err(Error::from("You already have 10 reminders in this guild."));
        }

        let current_time = get_seconds();
        let finish_time = current_time.saturating_add(duration);

        let remind_info = RemindInfo {
            original_time: current_time,
            request_time: current_time,
            finish_time,
            channel_id,
            guild_id,
            id: None,
            user_id,
            message_id: Some(message_id_callback().await?),
            message,
            looping,
        };

        locked_reminders_data.add_reminder(remind_info, &db).await?;
        drop(locked_reminders_data);

        let mut mut_wait_until = self.wait_until.lock().await;
        *mut_wait_until = mut_wait_until.min(finish_time);
        drop(mut_wait_until);

        return Ok(());
    }

    /// Removes reminder
    ///
    /// If guild_id is None it will remove a reminder from dms. Else it will remove a reminder from the guild.
    ///
    /// Will error if unable to communicate with db or if callback errors.
    pub async fn remove_reminder(
        &self,
        user_id: UserId,
        guild_id: Option<GuildId>,
        removal_index: usize,
    ) -> Result<RemindInfo, Error> {
        let db = &self.db;

        let reminders_data = self.get_reminders_data(user_id).await;
        let mut locked_reminders_data = reminders_data.lock().await;

        let removed_reminder = locked_reminders_data
            .delete_reminder(removal_index, guild_id, db)
            .await?;

        return Ok(removed_reminder);
    }
}

/// Main loop for checking if it's time for any reminders to be reminded.
pub fn remind_manager_loop(
    arc_ctx: Arc<Context>,
    remind_manager: Arc<RemindManager>,
    log_manager: Arc<LogManager>,
) {
    tokio::spawn(async move {
        let db = &remind_manager.db;

        let ttl_clear_cooldown = 30 * 60;

        let mut time_since_last_ttl_clear = 0;

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            time_since_last_ttl_clear += 1;

            if time_since_last_ttl_clear > ttl_clear_cooldown {
                time_since_last_ttl_clear = 0;
                remind_manager.reminders_data.write().await.clear_expired();
            }

            let current_time = get_seconds();

            let wait_until = remind_manager.wait_until.lock().await.clone();

            if wait_until > current_time {
                continue;
            }

            let result = db.get_pending_reminders(current_time).await;

            let mut reminders = match result {
                Ok(reminders) => reminders,
                Err(err) => {
                    let _ = log_manager
                        .add_owner_log(
                            format!("Couldn't fetch pending reminders. {}", err),
                            LogType::Warning,
                            LogSource::Reminder,
                        )
                        .await;
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    continue;
                }
            };
            for reminder_info in reminders.iter_mut() {
                if reminder_info.finish_time > current_time {
                    break;
                }

                let Some(reminder_id) = &reminder_info.id else {
                    //This should never happen.
                    continue;
                };

                let channel_id = reminder_info.channel_id;

                let message_ending;

                if let Some(message) = &reminder_info.message {
                    message_ending = format!(" with: {message}")
                } else {
                    message_ending = ".".to_string()
                }

                let time_difference = current_time - reminder_info.finish_time;

                let message_refrence_opt;

                if let Some(message_id) = reminder_info.message_id {
                    let mut message_refrence = MessageReference::from((channel_id, message_id));
                    if let Some(guild_id) = reminder_info.guild_id {
                        message_refrence.guild_id = Some(guild_id);
                    }
                    message_refrence_opt = Some(message_refrence);
                } else {
                    message_refrence_opt = None;
                }

                let mut create_message = CreateMessage::new().allowed_mentions(
                    CreateAllowedMentions::new().users(vec![reminder_info.user_id]),
                );

                if let Some(message_refrence) = message_refrence_opt {
                    create_message = create_message.reference_message(message_refrence);
                }

                if reminder_info.looping {
                    let wait_time = reminder_info.finish_time - reminder_info.request_time;
                    let missed_reminders =
                        (current_time - reminder_info.request_time) / wait_time - 1;

                    let res = if time_difference > 60 {
                        channel_id.send_message(arc_ctx.clone(), create_message.content(format!("Sorry <@!{}>, I was supposed to remind you <t:{}:R>! <t:{}:R> you told me to keep reminding you{message_ending}", reminder_info.user_id, reminder_info.finish_time, reminder_info.original_time))).await
                    } else {
                        channel_id.send_message(arc_ctx.clone(), create_message.content(format!("<@!{}>! <t:{}:R> you told me to keep reminding you{message_ending}", reminder_info.user_id, reminder_info.original_time))).await
                    };

                    if let Err(err) = res {
                        if is_user_fault(&err) {
                            let add_log = format!(
                                "Failed to send reminder. Deleting reminder. Reason: {}",
                                err.to_string()
                            );
                            if let Some(guild_id) = reminder_info.guild_id {
                                let id = IdType::GuildId(guild_id);
                                let _ = log_manager
                                    .add_log(
                                        id,
                                        add_log.clone(),
                                        LogType::Error,
                                        LogSource::Reminder,
                                    )
                                    .await;
                            }

                            let id = IdType::UserId(reminder_info.user_id);
                            let _ = log_manager
                                .add_log(id, add_log, LogType::Error, LogSource::Reminder)
                                .await;
                            let _ = db.delete_table_id(&reminder_id).await;
                        }
                        continue;
                    }

                    reminder_info.request_time =
                        reminder_info.finish_time + wait_time * missed_reminders;
                    reminder_info.finish_time = reminder_info.request_time + wait_time;

                    let json_string = serde_json::to_string(&reminder_info).unwrap();

                    loop {
                        let res = db
                            .query(format!("UPDATE {reminder_id} CONTENT {json_string}"))
                            .await;
                        if let Ok(ok) = res {
                            let Some(err) = ok.take_err(0) else {
                                break;
                            };
                            // this isnt a connection issue and shouldn't happen.
                            let _ = log_manager
                                .add_owner_log(
                                    format!("Failed to update looped reminder. {err}"),
                                    LogType::Warning,
                                    LogSource::Reminder,
                                )
                                .await;
                        }
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                } else {
                    let res = if time_difference > 60 {
                        channel_id.send_message(arc_ctx.clone(), create_message.content(format!("Sorry <@!{}>, I was supposed to remind you <t:{}:R>! <t:{}:R> you told me to remind you{message_ending}", reminder_info.user_id, reminder_info.finish_time, reminder_info.original_time))).await
                    } else {
                        channel_id
                            .send_message(
                                arc_ctx.clone(),
                                create_message.content(format!(
                                    "<@!{}>! <t:{}:R> you told me to remind you{message_ending}",
                                    reminder_info.user_id, reminder_info.original_time
                                )),
                            )
                            .await
                    };

                    if let Err(err) = res {
                        if !is_user_fault(&err) {
                            println!("{:?}", err);
                            continue;
                        } else {
                            let add_log = format!(
                                "Failed to send reminder. Deleting reminder. Reason: {}",
                                err.to_string()
                            );
                            if let Some(guild_id) = reminder_info.guild_id {
                                let id = IdType::GuildId(guild_id);
                                let _ = log_manager
                                    .add_log(
                                        id,
                                        add_log.clone(),
                                        LogType::Error,
                                        LogSource::Reminder,
                                    )
                                    .await;
                            }

                            let id = IdType::UserId(reminder_info.user_id);
                            let _ = log_manager
                                .add_log(id, add_log, LogType::Error, LogSource::Reminder)
                                .await;
                        }
                    }

                    loop {
                        let res = db.delete_table_id(reminder_id).await;
                        if res.is_ok() {
                            break;
                        }
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }

                // Reset the cache for the user whose reminder just happened.
                remind_manager
                    .get_reminders_data(reminder_info.user_id)
                    .await
                    .lock()
                    .await
                    .reminders = None;
            }

            let next_wait_until;

            loop {
                next_wait_until = match db.get_next_reminder_time().await {
                    Ok(next_time) => next_time,
                    Err(err) => {
                        let _ = log_manager
                            .add_owner_log(
                                format!("Couldn't fetch next reminder time. {}", err),
                                LogType::Warning,
                                LogSource::Reminder,
                            )
                            .await;
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        continue;
                    }
                };
                break;
            }

            *remind_manager.wait_until.lock().await = next_wait_until.unwrap_or(u64::MAX);
        }
    });
}

/// Checks if a serenity error is due to a user issue for example bot role perms, missing guild or channel.
pub fn is_user_fault(error: &poise::serenity_prelude::Error) -> bool {
    match error {
        poise::serenity_prelude::Error::Http(err) => match err {
            poise::serenity_prelude::HttpError::UnsuccessfulRequest(err) => {
                //https://discord.com/developers/docs/topics/opcodes-and-status-codes#http
                const FORBIDDEN: u16 = 403;
                const NOT_FOUND: u16 = 404;
                const METHOD_NOT_ALLOWED: u16 = 405;
                return err.status_code == FORBIDDEN
                    || err.status_code == NOT_FOUND
                    || err.status_code == METHOD_NOT_ALLOWED;
            }
            _ => false,
        },
        _ => false,
    }
}
