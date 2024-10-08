use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Context, Error};

/// Pong!
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    let current = get_current_ms_time();
    ctx.defer().await?;
    let after = get_current_ms_time();
    let difference = after - current;
    ctx.say(format!("Pong! `{}ms`", difference)).await?;
    Ok(())
}

pub fn get_current_ms_time() -> u128 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards. Oopsie.");
    since_the_epoch.as_millis()
}
