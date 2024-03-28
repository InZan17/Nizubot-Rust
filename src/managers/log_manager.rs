use std::{
    borrow::BorrowMut,
    collections::HashMap,
    hash::Hash,
    mem::transmute,
    ops::Add,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use chrono::Timelike;
use openssl::pkey::Id;
use poise::serenity_prelude::{self, Context, UserId, Webhook};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{Mutex, RwLock},
};

use crate::{
    managers::{cotd_manager::SECONDS_IN_A_DAY, storage_manager::DataType},
    utils::{get_seconds, IdType},
    Error,
};

use super::{
    db::SurrealClient,
    storage_manager::{DataHolder, StorageManager},
};

pub struct LogState {
    logs: Vec<String>,
    needs_saving: bool,
    last_used: u64,
}

impl LogState {
    pub fn blank() -> Self {
        Self {
            logs: vec![],
            needs_saving: false,
            last_used: get_seconds(),
        }
    }

    pub fn add_log(&mut self, add_log: String) {
        let now = chrono::Utc::now();
        let date = now.to_rfc2822();
        let log = format!("[{date}] {add_log}",);
        self.logs.push(log);
        self.needs_saving = true;
        self.last_used = get_seconds();
    }

    pub fn clear(&mut self) {
        self.logs = vec![];
        self.needs_saving = true;
    }
}

pub struct LogManager {
    db: Arc<SurrealClient>,
    log_path: PathBuf,
    owner_user_ids: Vec<UserId>,
    admin_log_webhook: Option<Webhook>,
    arc_ctx: Arc<Context>,
    logs: RwLock<HashMap<IdType, Mutex<LogState>>>,
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
            LogSource::LogManager => "LOG_MANAGER",
            LogSource::Custom(string) => string,
        }
    }
}

impl LogManager {
    pub async fn load_state(&self, id: &IdType) -> LogState {
        let mut new_path = self.log_path.clone();
        new_path.push(LogManager::get_file_name(id));

        let mut log_state = LogState::blank();

        let Ok(mut file) = tokio::fs::File::open(new_path).await else {
            return log_state
        };

        let mut string = String::new();

        if file.read_to_string(&mut string).await.is_err() {
            return log_state;
        }

        log_state.logs.push(string);

        return log_state;
    }

    pub fn new(
        db: Arc<SurrealClient>,
        log_path: PathBuf,
        owner_user_ids: Vec<UserId>,
        admin_log_webhook: Option<Webhook>,
        arc_ctx: Arc<Context>,
    ) -> Self {
        Self {
            db,
            log_path,
            owner_user_ids,
            admin_log_webhook,
            arc_ctx,
            logs: RwLock::new(HashMap::new()),
        }
    }

    async fn write_log(&self, id: &IdType, data: &str) -> Result<(), Error> {
        let mut new_path = self.log_path.clone();
        new_path.push(LogManager::get_file_name(id));

        tokio::fs::create_dir_all(&self.log_path).await?;

        let mut file = tokio::fs::File::create(new_path).await?;
        file.write_all(data.as_bytes()).await?;

        Ok(())
    }

    async fn clear_unused_logs(&self) {
        let mut write = self.logs.write().await;
        let seconds = get_seconds();
        let mut log_removals = vec![];

        let mut err_count = 0;
        let mut save_count = 0;
        let mut latest_err = "".into();

        for (id, log_state) in write.iter() {
            let mut lock_log = log_state.lock().await;

            if lock_log.last_used + SECONDS_IN_A_DAY > seconds {
                continue;
            }

            if lock_log.needs_saving {
                save_count += 1;

                let joined = lock_log.logs.join("\n");

                if let Err(err) = self.write_log(id, &joined).await {
                    latest_err = err;
                    err_count += 1;
                    continue;
                }

                lock_log.needs_saving = false;
            }
            log_removals.push(id.clone());
        }

        for id in log_removals {
            write.remove(&id);
        }

        if err_count > 0 {
            self.add_owner_log(format!("Failed to save {err_count} out of {save_count} logs. Latest error: {latest_err}"), LogType::Error, LogSource::LogManager).await;
        }
    }

    pub fn get_file_name(id: &IdType) -> String {
        match id {
            IdType::UserId(user_id) => format!("user_{user_id}.log"),
            IdType::GuildId(guild_id) => format!("guild_{guild_id}.log"),
        }
    }

    pub async fn get_logs(&self, id: &IdType) -> Result<String, Error> {
        let mut logs_read = self.logs.read().await;
        let log_state = {
            if let Some(log_state) = logs_read.get(id) {
                log_state
            } else {
                drop(logs_read);
                let mut logs_write = self.logs.write().await;
                logs_write.insert(id.clone(), Mutex::new(self.load_state(id).await));
                drop(logs_write);
                logs_read = self.logs.read().await;
                logs_read.get(id).unwrap_or_else(|| panic!())
            }
        };

        let read = log_state.lock().await;

        let logs = read.logs.join("\n");

        Ok(logs)
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

        let arc_ctx = self.arc_ctx.clone();

        if let Some(webhook) = &self.admin_log_webhook {
            let _ = webhook
                .execute(arc_ctx, false, |m| {
                    m.content(Self::create_log_string(add_log, log_type, log_source))
                })
                .await;
        }
        result
    }

    pub fn create_log_string(add_log: String, log_type: LogType, log_source: LogSource) -> String {
        format!("[{}:{}] {add_log}", log_source.to_str(), log_type.to_str())
    }

    pub async fn add_log(
        &self,
        id: &IdType,
        add_log: String,
        log_type: LogType,
        log_source: LogSource,
    ) -> Result<(), Error> {
        let mut logs_read = self.logs.read().await;
        let log_state = {
            if let Some(log_state) = logs_read.get(id) {
                log_state
            } else {
                drop(logs_read);
                let mut logs_write = self.logs.write().await;
                logs_write.insert(id.clone(), Mutex::new(self.load_state(id).await));
                drop(logs_write);
                logs_read = self.logs.read().await;
                logs_read.get(id).unwrap_or_else(|| panic!())
            }
        };

        let add_str = format!("[{}:{}] {add_log}", log_type.to_str(), log_source.to_str());

        let mut log_state_lock = log_state.lock().await;
        log_state_lock.add_log(add_str);

        drop(log_state_lock);

        Ok(())
    }

    pub async fn clear_log(&self, id: &IdType) -> Result<(), Error> {
        let mut logs_read = self.logs.read().await;
        let log_state = {
            if let Some(log_state) = logs_read.get(id) {
                log_state
            } else {
                drop(logs_read);
                let mut logs_write = self.logs.write().await;
                logs_write.insert(id.clone(), Mutex::new(self.load_state(id).await));
                drop(logs_write);
                logs_read = self.logs.read().await;
                logs_read.get(id).unwrap_or_else(|| panic!())
            }
        };

        log_state.lock().await.clear();
        Ok(())
    }
}

pub fn log_manager_loop(_arc_ctx: Arc<Context>, log_manager: Arc<LogManager>) {
    tokio::spawn(async move {
        let mut loop_state = 0;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

            loop_state += 1;

            if loop_state == 60 {
                log_manager.clear_unused_logs().await;
                loop_state = 0;
                continue;
            }

            let read = log_manager.logs.read().await;

            let mut save_count = 0;
            let mut err_count = 0;
            let mut latest_err = "".into();

            for (id, state) in read.iter() {
                let mut lock_log = state.lock().await;
                if lock_log.needs_saving {
                    save_count += 1;

                    let joined = lock_log.logs.join("\n");

                    if let Err(err) = log_manager.write_log(id, &joined).await {
                        latest_err = err;
                        err_count += 1;
                        continue;
                    }

                    lock_log.needs_saving = false;
                }
            }

            if err_count > 0 {
                log_manager.add_owner_log(format!("Failed to save {err_count} out of {save_count} logs. Latest error: {latest_err}"), LogType::Error, LogSource::LogManager).await;
            }
        }
    });
}
