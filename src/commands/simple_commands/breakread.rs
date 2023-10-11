use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{managers::storage_manager::DataInfo, Context, Error};
use poise::serenity_prelude::RwLock;

/// Break Read!
#[poise::command(slash_command)]
pub async fn breakread(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data();
    let data = data
        .storage_manager
        .get_data::<i32>(vec!["heck"])
        .await
        .unwrap();
    let read2 = data.read().await;
    let read = read2.get_data().clone();
    drop(read2);
    ctx.say(format!("time to break myself: {}", read)).await?;
    Ok(())
}
