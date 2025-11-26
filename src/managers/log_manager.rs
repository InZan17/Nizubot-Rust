use std::{collections::VecDeque, sync::Arc, time::Duration};

use chrono::{DateTime, TimeDelta, Utc};
use poise::serenity_prelude::{
    Builder, CacheHttp, ChannelId, Context, CreateAttachment, CreateWebhook, ExecuteWebhook,
    UserId, Webhook,
};
use tokio::{
    sync::{Mutex, RwLock},
    task::spawn_blocking,
};

use crate::{
    managers::storage_manager::{DataType, StorageManager},
    utils::{IdType, TtlMap},
    Error,
};

pub struct LogState {
    logs: VecDeque<(DateTime<Utc>, DateTime<Utc>, String, usize)>,
    log_lifetime: Duration,
    merge_window: Duration,
    max_capacity: usize,
    id: IdType,
    webhook: Option<(Webhook, DateTime<Utc>, usize)>,
}

impl LogState {
    pub fn new(id: IdType) -> Self {
        Self {
            logs: VecDeque::new(),
            log_lifetime: Duration::from_secs(60 * 60 * 12),
            merge_window: Duration::from_secs(60 * 10),
            max_capacity: 5000,
            id,
            webhook: None,
        }
    }

    pub fn set_webhook_silent(&mut self, webhook: Webhook) {
        self.webhook = Some((webhook, Utc::now(), 0));
    }

    pub async fn set_webhook(
        &mut self,
        webhook: Webhook,
        cache_http: impl CacheHttp,
    ) -> Result<(), Error> {
        let now = Utc::now();
        let message = LogManager::create_log_string(
            "Successfully created webhook for logging.",
            LogType::Info,
            LogSource::LogManager,
        );

        let formatted = format_log(now, now, &message, 1);

        webhook
            .execute(cache_http, false, ExecuteWebhook::new().content(formatted))
            .await?;

        self.add_raw(message);
        self.webhook = Some((webhook, Utc::now(), 0));
        Ok(())
    }

    pub fn add(&mut self, message: String, log_type: LogType, log_source: LogSource) {
        self.add_raw(LogManager::create_log_string(
            &message, log_type, log_source,
        ));
    }

    pub fn add_raw(&mut self, message: String) {
        let message = message.into();
        let now = Utc::now();

        if let Some((_, _, count)) = &mut self.webhook {
            *count += 1;
        }

        if let Some((first_timestamp, last_timestamp, previous_message, count)) =
            self.logs.back_mut()
        {
            if *previous_message == message
                && now.signed_duration_since(first_timestamp)
                    <= TimeDelta::from_std(self.merge_window).unwrap()
            {
                *last_timestamp = now;
                *count += 1;
                return;
            }
        }

        if self.logs.len() >= self.max_capacity {
            self.logs.pop_front();
        }

        self.logs.push_back((now, now, message, 1));
    }

    pub fn clear_expired(&mut self) {
        let expired_before = Utc::now() - self.log_lifetime;

        while let Some((_first_timestamp, last_timestamp, _, _)) = self.logs.front() {
            if *last_timestamp < expired_before {
                self.logs.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn get_formatted(&self) -> String {
        let expired_before = Utc::now() - self.log_lifetime;

        self.logs
            .iter()
            .filter(|(_first_timestamp, last_timestamp, _, _)| *last_timestamp >= expired_before)
            .map(|(first_timestamp, last_timestamp, message, count)| {
                format_log(*first_timestamp, *last_timestamp, message, *count)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub fn format_log(
    first_timestamp: DateTime<Utc>,
    last_timestamp: DateTime<Utc>,
    message: &str,
    count: usize,
) -> String {
    let first_time = first_timestamp.format("%d-%m-%Y %H-%M:%S%.3f").to_string();
    if first_timestamp == last_timestamp {
        if count > 1 {
            format!("[{first_time}] {message} (x{count})")
        } else {
            format!("[{first_time}] {message}")
        }
    } else {
        let last_time = last_timestamp.format("%d-%m-%Y %H-%M:%S%.3f").to_string();
        if count > 1 {
            format!("[{first_time} -> {last_time}] {message} (x{count})")
        } else {
            format!("[{first_time} -> {last_time}] {message}")
        }
    }
}

pub struct LogManager {
    storage_manager: Arc<StorageManager>,
    owner_user_ids: Vec<UserId>,
    arc_ctx: Arc<Context>,
    logs: RwLock<TtlMap<IdType, Arc<Mutex<LogState>>>>,
    logs_with_webhooks: Mutex<Vec<Arc<Mutex<LogState>>>>,
}

#[derive(Clone, Copy)]
pub enum LogType {
    Info,
    Warning,
    Error,
}

impl LogType {
    pub fn to_str(&self) -> &str {
        match self {
            LogType::Info => "INFO",
            LogType::Warning => "WARNING",
            LogType::Error => "ERROR",
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
    Lua,
    LogManager,
    Custom(String),
}

impl LogSource {
    pub fn to_str(&self) -> &str {
        match self {
            LogSource::Guild => "GUILD",
            LogSource::User => "USER",
            LogSource::MessageDetector => "MESSAGE_DETECTOR",
            LogSource::ReactionRole => "REACTION_ROLE",
            LogSource::CotdRole => "COTD_ROLE",
            LogSource::Reminder => "REMINDER",
            LogSource::Lua => "LUA",
            LogSource::LogManager => "LOG_MANAGER",
            LogSource::Custom(string) => string,
        }
    }
}

impl LogManager {
    pub fn new(
        storage_manager: Arc<StorageManager>,
        owner_user_ids: Vec<UserId>,
        arc_ctx: Arc<Context>,
    ) -> Self {
        Self {
            storage_manager,
            owner_user_ids,
            arc_ctx,
            logs: RwLock::new(TtlMap::new(Duration::from_secs(60 * 60 * 12))),
            logs_with_webhooks: Mutex::new(Vec::new()),
        }
    }

    async fn get_log_state(&self, id: IdType) -> Arc<Mutex<LogState>> {
        if let Some(log_state) = self.logs.read().await.get(&id).cloned() {
            return log_state;
        }

        let mut guild_data_mut = self.logs.write().await;
        if let Some(log_state) = guild_data_mut.get(&id).cloned() {
            return log_state;
        }

        let mut log_state = LogState::new(id);

        let result = self
            .storage_manager
            .load_disk(
                &format!(
                    "{}/{}/webhook_log.json",
                    if id.is_user() { "user" } else { "guild" },
                    id.get_u64()
                ),
                true,
            )
            .await;

        match result {
            Ok(Some(data)) => {
                let string = data.string().unwrap_or("");
                let result = serde_json::from_str(string);
                match result {
                    Ok(webhook) => log_state.set_webhook_silent(webhook),
                    Err(err) => log_state.add(
                        format!("Failed to parse webhook data from disk.\n{err}"),
                        LogType::Error,
                        LogSource::LogManager,
                    ),
                };
            }
            Err(err) => log_state.add(
                format!("Failed to load webhook data from disk.\n{err}"),
                LogType::Error,
                LogSource::LogManager,
            ),
            Ok(None) => {}
        }

        let has_webhook = log_state.webhook.is_some();

        let log_state = Arc::new(Mutex::new(log_state));

        if has_webhook {
            self.logs_with_webhooks.lock().await.push(log_state.clone());
        }

        guild_data_mut.insert(id, log_state.clone());

        log_state
    }

    async fn try_get_log_state(&self, id: IdType) -> Option<Arc<Mutex<LogState>>> {
        self.logs.read().await.get(&id).cloned()
    }

    pub fn get_file_name(id: IdType) -> String {
        match id {
            IdType::UserId(user_id) => format!("user_{user_id}.log"),
            IdType::GuildId(guild_id) => format!("guild_{guild_id}.log"),
        }
    }

    pub async fn get_formatted(&self, id: IdType) -> String {
        let Some(log_state) = self.try_get_log_state(id).await else {
            return String::new();
        };

        let lock = log_state.lock().await;
        lock.get_formatted()
    }

    pub async fn add_owner_log(&self, add_log: String, log_type: LogType, log_source: LogSource) {
        for owner_id in self.owner_user_ids.iter() {
            self.add_log(
                IdType::UserId(*owner_id),
                add_log.clone(),
                log_type,
                log_source.clone(),
            )
            .await;
        }
    }

    pub fn create_log_string(add_log: &str, log_type: LogType, log_source: LogSource) -> String {
        format!("[{}:{}] {add_log}", log_source.to_str(), log_type.to_str())
    }

    pub async fn add_log(
        &self,
        id: IdType,
        message: String,
        log_type: LogType,
        log_source: LogSource,
    ) {
        let log_state = self.get_log_state(id).await;
        let mut log_state_lock = log_state.lock().await;
        log_state_lock.add(message, log_type, log_source);
    }

    pub async fn add_webhook(
        &self,
        id: IdType,
        webhook_or_channel: either::Either<Webhook, (ChannelId, String)>,
    ) -> Result<(), Error> {
        let log_state = self.get_log_state(id).await;
        let mut log_state_lock = log_state.lock().await;

        if log_state_lock.webhook.is_some() {
            return Err(
                "It seems you already have a webhook connected. Consider removing it first if you wanna change your webhook.".into(),
            );
        }

        let webhook = match webhook_or_channel {
            either::Either::Left(webhook) => webhook,
            either::Either::Right((channel_id, bot_name)) => {
                CreateWebhook::new(format!("{bot_name} Logs"))
                    .execute(&self.arc_ctx, channel_id)
                    .await?
            }
        };

        let webhook_string = serde_json::to_string(&webhook)?;

        log_state_lock.set_webhook(webhook, &self.arc_ctx).await?;
        self.logs_with_webhooks.lock().await.push(log_state.clone());

        if let Err(err) = self
            .storage_manager
            .save_disk(
                &format!(
                    "{}/{}/webhook_log.json",
                    if id.is_user() { "user" } else { "guild" },
                    id.get_u64()
                ),
                &DataType::String(webhook_string),
            )
            .await
        {
            let err_message = format!(
                    "Failed to save webhook to disk. Webhook will stop logging after a few hours. Consider running the command again until it succeeds or contact the bot creator.\n{err}"
                );
            log_state_lock.add(err_message.clone(), LogType::Error, LogSource::LogManager);
            return Err(err_message.into());
        };
        Ok(())
    }

    pub async fn remove_webhook(&self, id: IdType) -> Result<(), Error> {
        let log_state = self.get_log_state(id).await;
        let mut log_state_lock = log_state.lock().await;
        log_state_lock.webhook = None;
        if let Err(err) = self
            .storage_manager
            .delete_disk(&format!(
                "{}/{}/webhook_log.json",
                if id.is_user() { "user" } else { "guild" },
                id.get_u64()
            ))
            .await
        {
            let err_message = format!(
                    "Failed to remove webhook from disk. Webhook will start logging again after a few hours. Consider running the command again until it succeeds or delete the webhook through Discord or contact the bot creator.\n{err}"
                );
            log_state_lock.add(err_message.clone(), LogType::Error, LogSource::LogManager);
            return Err(err_message.into());
        };
        Ok(())
    }
}

pub fn log_manager_loop(arc_ctx: Arc<Context>, log_manager: Arc<LogManager>) {
    tokio::spawn(async move {
        let mut loop_count = 0;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            loop_count += 1;

            {
                let log_manager2 = log_manager.clone();
                let _ = spawn_blocking(move || {
                    let mut logs_with_webhook_lock =
                        log_manager2.logs_with_webhooks.blocking_lock();
                    logs_with_webhook_lock.retain(|log| {
                        Arc::strong_count(log) > 1 && log.blocking_lock().webhook.is_some()
                    });
                })
                .await;
            }

            let logs = log_manager.logs_with_webhooks.lock().await.clone();

            let now = Utc::now();

            for log in logs {
                let mut log_lock = log.lock().await;
                let id = log_lock.id;
                let Some((_, last_used, missed_logs)) = &mut log_lock.webhook else {
                    continue;
                };

                let mut updated_missed_logs = *missed_logs;
                let last_used = *last_used;

                if updated_missed_logs > 0
                    && now.signed_duration_since(last_used)
                        > TimeDelta::from_std(Duration::from_secs(2)).unwrap()
                {
                    let mut pending_logs = Vec::new();

                    for (first_timestamp, last_timestamp, message, count) in log_lock.logs.iter() {
                        let new_missed_logs = updated_missed_logs.saturating_sub(*count);
                        let new_count = updated_missed_logs.min(*count);
                        let new_first_timestamp = if new_missed_logs == 0 && *count != new_count {
                            if new_count == 1 {
                                *last_timestamp
                            } else {
                                last_used
                            }
                        } else {
                            *first_timestamp
                        };

                        pending_logs.push(format_log(
                            new_first_timestamp,
                            *last_timestamp,
                            message,
                            new_count,
                        ));

                        updated_missed_logs = new_missed_logs;

                        if updated_missed_logs == 0 {
                            break;
                        }
                    }

                    let Some((webhook, last_used, missed_logs)) = &mut log_lock.webhook else {
                        continue;
                    };

                    let message = pending_logs.join("\n");

                    let result = if message.len() > 1000 {
                        webhook
                            .execute(
                                &arc_ctx,
                                false,
                                ExecuteWebhook::new().add_file(CreateAttachment::bytes(
                                    message,
                                    LogManager::get_file_name(id),
                                )),
                            )
                            .await
                    } else {
                        webhook
                            .execute(&arc_ctx, false, ExecuteWebhook::new().content(message))
                            .await
                    };

                    *last_used = Utc::now();
                    *missed_logs = 0;

                    if let Err(err) = result {
                        log_lock.add(err.to_string(), LogType::Error, LogSource::LogManager);
                    }
                }
            }

            if loop_count > 30 * 60 {
                loop_count = 0;
                log_manager.logs.write().await.clear_expired();

                let logs_read = log_manager.logs.read().await;
                for (_, log_state) in logs_read.silent_iter() {
                    log_state.lock().await.clear_expired();
                }
            }
        }
    });
}
