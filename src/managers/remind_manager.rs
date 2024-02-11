use core::panic;
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
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::Error;

use super::db::SurrealClient;

pub struct RemindManager {
    db: Arc<SurrealClient>,
    pub wait_until: Mutex<u64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RemindInfo {
    pub original_time: u64,
    pub request_time: u64,
    pub finish_time: u64,
    pub channel_id: u64,
    pub guild_id: Option<u64>,
    pub user_id: u64,
    /// id will be Some() when retrieved from surrealdb. Otherwise None.
    #[serde(skip_serializing)]
    pub id: Option<String>,
    pub message_id: Option<u64>,
    pub message: Option<String>,
    pub looping: bool,
}

impl RemindManager {
    pub fn new(db: Arc<SurrealClient>) -> Self {
        RemindManager {
            db,
            wait_until: Mutex::new(0),
        }
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
    pub async fn add_reminder<'a, F, Fut>(
        &self,
        guild_id: Option<u64>,
        channel_id: u64,
        user_id: UserId,
        duration: u64,
        looping: bool,
        message: Option<String>,
        // Callback that returns the message id of the message bot should reply to.
        message_id_callback: F,
    ) -> Result<(), Error>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<u64, Error>>,
    {
        let db = &self.db;

        let user_table_id = format!("user:{user_id}");

        //TODO: Put in seperate function
        let user_reminders: Vec<RemindInfo> = db
            .query(format!(
                "
            LET $reminders = SELECT VALUE ->reminds->reminder FROM {user_table_id};

            IF array::len($reminders) THEN
                SELECT * FROM array::first($reminders) ORDER BY original_time;
            ELSE
                RETURN [];
            END
        "
            ))
            .await?
            .take(1)?;

        if user_reminders.len() >= 50 {
            return Err(Error::from(
                "You've already got a total of 50 reminders. Consider removing some.",
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
        let finish_time = current_time + duration;

        let remind_info = RemindInfo {
            original_time: current_time,
            request_time: current_time,
            finish_time,
            channel_id,
            guild_id,
            id: None,
            user_id: *user_id.as_u64(),
            message_id: Some(message_id_callback().await?),
            message,
            looping,
        };

        let remind_info_json = serde_json::to_string(&remind_info)?;

        // TODO: Put this logic and query in a seperate function
        let guild_relate_statement = if let Some(guild_id) = guild_id {
            let guild_table_id = format!("guild:{guild_id}");
            format!(
                "
            UPDATE {guild_table_id};
            RELATE {guild_table_id}->reminds->$reminder;
            "
            )
        } else {
            "RETURN;RETURN;".to_owned()
        };

        self.db
            .query(format!(
                "
        BEGIN TRANSACTION;

        LET $reminder = (CREATE reminder CONTENT {remind_info_json});

        UPDATE {user_table_id};
        RELATE {user_table_id}->reminds->$reminder;
        {guild_relate_statement}

        COMMIT TRANSACTION;
        "
            ))
            .await?;

        //TODO: add remidner and also fix the index

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
        user_id: u64,
        guild_id: Option<u64>,
        removal_index: usize,
    ) -> Result<Option<RemindInfo>, Error> {
        let db = &self.db;

        let table_id = format!("user:{user_id}");

        //TODO: put in funtion
        let mut reminders: Vec<RemindInfo> = db
            .query(format!(
                "
            LET $reminders = SELECT VALUE ->reminds->reminder FROM {table_id};

            IF array::len($reminders) THEN
                SELECT * FROM array::first($reminders) ORDER BY original_time;
            ELSE
                RETURN [];
            END
        "
            ))
            .await?
            .take(1)?;

        let mut reminders_index = 0;
        let mut reminders_guild_index = 0;
        let mut found = false;

        for (index, reminder) in reminders.iter().enumerate() {
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
            return Ok(None);
        }

        let removed_reminder = reminders.swap_remove(reminders_index);

        let Some(reminder_id) = &removed_reminder.id else {
            return Err("Reminder didn't have a database id.".into());
        };

        db.query(format!("DELETE {reminder_id}")).await?;

        return Ok(Some(removed_reminder));
    }

    /// Lists reminders
    ///
    /// If guild_id is None it will list reminders from dms. Else it will list reminders from the guild.
    ///
    /// Will error if unable to communicate with db or if callback errors.
    pub async fn list_reminders(
        &self,
        user_id: u64,
        guild_id: Option<u64>,
    ) -> Result<Vec<RemindInfo>, Error> {
        let db = &self.db;

        let table_id = format!("user:{user_id}");

        //TODO put in function
        let reminders: Vec<RemindInfo> = db
            .query(format!(
                "
            LET $reminders = SELECT VALUE ->reminds->reminder FROM {table_id};

            IF array::len($reminders) THEN
                SELECT * FROM array::first($reminders) ORDER BY original_time;
            ELSE
                RETURN [];
            END
        "
            ))
            .await?
            .take(1)?;

        let mut specific_reminders = vec![];

        for reminder in reminders.iter() {
            if reminder.guild_id == guild_id {
                specific_reminders.push(reminder.clone());
            }
        }

        Ok(specific_reminders)
    }
}

fn get_seconds() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards. Oopsie.");

    since_the_epoch.as_secs()
}

/// Main loop for checking if it's time for any reminders to be reminded.
pub fn remind_manager_loop(arc_ctx: Arc<Context>, remind_manager: Arc<RemindManager>) {
    tokio::spawn(async move {
        let db = &remind_manager.db;

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let current_time = get_seconds();

            let wait_until = remind_manager.wait_until.lock().await.clone();

            if wait_until > current_time {
                continue;
            }

            let mut next_wait_until = u64::MAX;

            let mut query_response = match db
                .query(format!(
                    "SELECT * FROM reminder WHERE finish_time <= {current_time};"
                ))
                .await
            {
                Ok(response) => response,
                Err(err) => {
                    panic!("{err}");
                    //TODO; Do somethign with the error. Maybe use a log for the bot specifically.
                    continue;
                }
            };

            let mut reminders: Vec<RemindInfo> = match query_response.take(0) {
                Ok(reminders) => reminders,
                Err(err) => {
                    panic!("{err}");
                    //TODO; Do somethign with the error. Maybe use a log for the bot specifically.
                    continue;
                }
            };
            for (index, reminder_info) in reminders.iter_mut().enumerate() {
                if reminder_info.finish_time > current_time {
                    next_wait_until = next_wait_until.min(reminder_info.finish_time);
                    break;
                }

                let Some(reminder_id) = &reminder_info.id else {
                    //TODO: notify or soeming;
                    continue;
                };

                let channel_id = ChannelId(reminder_info.channel_id);

                let message_ending;

                if let Some(message) = &reminder_info.message {
                    message_ending = format!(" with: {message}")
                } else {
                    message_ending = ".".to_string()
                }

                //TODO: Put this in a loop.

                //First we try to send the reminder message.
                //If it fails we check if it's a discord permission issue.
                //If it is we remove the reminder from database and put something on the users log.
                //If removing it from database doesn't work then I guess we'll let it slide.
                //If it isn't a discord permission issue then we'll continue the loop of sending the message.
                //If message sent successfully we delete from database (in a loop).
                //If database removal fails we redo the loop until it succeeds.
                //All reminders will be halted and if restarted there will be a double reminder for someone.

                let time_difference = current_time - reminder_info.finish_time;

                let message_refrence_opt;

                if let Some(message_id) = reminder_info.message_id {
                    let mut message_refrence =
                        MessageReference::from((channel_id, MessageId(message_id)));
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

                    let res = if time_difference > 60 {
                        channel_id.send_message(arc_ctx.clone(), |m| {
                            if let Some(message_refrence) = message_refrence_opt {
                                m.reference_message(message_refrence);
                            }
                            m.allowed_mentions(|a| {a.users(vec![reminder_info.user_id])}).content(format!("Sorry <@!{}>, I was supposed to remind you <t:{}:R>! <t:{}:R> you told me to keep reminding you{message_ending}", reminder_info.user_id, reminder_info.finish_time, reminder_info.original_time))}).await
                    } else {
                        channel_id.send_message(arc_ctx.clone(), |m| {
                            if let Some(message_refrence) = message_refrence_opt {
                                m.reference_message(message_refrence);
                            }
                            m.allowed_mentions(|a| {a.users(vec![reminder_info.user_id])}).content(format!("<@!{}>! <t:{}:R> you told me to keep reminding you{message_ending}", reminder_info.user_id, reminder_info.original_time))}).await
                    };

                    if let Err(err) = res {
                        if should_keep(err) {
                            next_wait_until = 0;
                        } else {
                            //TODO: notify to the user/server log.
                            //If a deletion fails, abort the remind manager loop because else it will just keep spamming the same reminders.
                            db.query(format!("DELETE {reminder_id}"))
                                .await
                                .unwrap()
                                .take::<Vec<Value>>(0)
                                .unwrap();
                        }
                        continue;
                    }

                    reminder_info.request_time =
                        reminder_info.finish_time + wait_time * missed_reminders;
                    reminder_info.finish_time = reminder_info.request_time + wait_time;

                    next_wait_until = next_wait_until.min(reminder_info.finish_time);

                    let json_string = serde_json::to_string(&reminder_info).unwrap();

                    let a: Option<RemindInfo> = db
                        .query(format!("UPDATE {reminder_id} CONTENT {json_string}"))
                        .await
                        .unwrap()
                        .take(0)
                        .unwrap();
                } else {
                    let res = if time_difference > 60 {
                        channel_id.send_message(arc_ctx.clone(), |m| {
                            if let Some(message_refrence) = message_refrence_opt {
                                m.reference_message(message_refrence);
                            }
                            m.allowed_mentions(|a| {a.users(vec![reminder_info.user_id])}).content(format!("Sorry <@!{}>, I was supposed to remind you <t:{}:R>! <t:{}:R> you told me to remind you{message_ending}", reminder_info.user_id, reminder_info.finish_time, reminder_info.original_time))}).await
                    } else {
                        channel_id.send_message(arc_ctx.clone(), |m| {
                            if let Some(message_refrence) = message_refrence_opt {
                                m.reference_message(message_refrence);
                            }
                            m.allowed_mentions(|a| {a.users(vec![reminder_info.user_id])}).content(format!("<@!{}>! <t:{}:R> you told me to remind you{message_ending}", reminder_info.user_id, reminder_info.original_time))}).await
                    };

                    if let Err(err) = res {
                        if should_keep(err) {
                            next_wait_until = 0;
                            continue;
                        } else {
                            //TODO: Notify to the user/server log
                        }
                    }

                    //If a deletion fails, abort the remind manager loop because else it will just keep spamming the same reminders.
                    let a = db
                        .query(format!("DELETE {reminder_id}"))
                        .await
                        .unwrap()
                        .take::<Vec<Value>>(0)
                        .unwrap();
                }
            }
            *remind_manager.wait_until.lock().await = next_wait_until;
        }
    });
}

/// Checks if a serenity error is due to internet issues (true) or discord issue for example bot role perms, missing guild or channel (false)
fn should_keep(error: poise::serenity_prelude::Error) -> bool {
    match error {
        poise::serenity_prelude::Error::Http(http) => match *http {
            poise::serenity_prelude::HttpError::Request(req) => {
                req.is_request() || req.is_timeout()
            }
            _ => false,
        },
        _ => false,
    }
}
