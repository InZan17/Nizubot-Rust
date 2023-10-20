use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{managers::storage_manager::DataHolder, Context, Error};
use poise::serenity_prelude::RwLock;

/// Read!
#[poise::command(slash_command)]
pub async fn read(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data();
    let data = data.storage_manager.get_data_or_default::<String>(vec!["heck"], "Nothing".to_owned()).await;
    
    ctx.say(format!("Written data: {}", *data.get_data().await)).await?;
    Ok(())
}
