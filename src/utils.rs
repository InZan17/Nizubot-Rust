use std::time::{SystemTime, UNIX_EPOCH};

use poise::serenity_prelude::{GuildId, UserId};

pub fn get_seconds() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards. Oopsie.");

    since_the_epoch.as_secs()
}

pub enum IdType {
    UserId(UserId),
    GuildId(GuildId),
}

impl IdType {
    pub fn into_db_table(&self) -> String {
        match self {
            IdType::UserId(user_id) => format!("user:{user_id}"),
            IdType::GuildId(guild_id) => format!("guild:{guild_id}"),
        }
    }
}
