use std::{fs, path::PathBuf};

use poise::serenity_prelude::UserId;
use serde::Deserialize;

pub const BOT_SETTINGS_FILE: &str = "./bot_settings.json";

#[derive(Debug, Deserialize)]
pub struct BotSettings {
    pub discord_token: String,
    pub open_exchange_rates_token: Option<String>,
    pub owner_user_ids: Vec<UserId>,
    pub temp_data_directory: PathBuf,
    pub surrealdb: SurrealDbSignInInfo,
}

#[derive(Debug, Deserialize)]
pub struct SurrealDbSignInInfo {
    pub address: String,
    pub namespace: String,
    pub database: String,
    pub username: String,
    pub password: String,
}

pub fn get_bot_settings() -> BotSettings {
    let json_data =
        fs::read_to_string(BOT_SETTINGS_FILE).expect("Couldn't read bot settings file.");
    serde_json::from_str(&json_data).expect("Couldn't deserialize bot settings file.")
}
