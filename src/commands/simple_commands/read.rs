use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{managers::storage_manager::DataInfo, Context, Error};
use poise::serenity_prelude::RwLock;

/// Read!
#[poise::command(slash_command)]
pub async fn read(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data();
    let data: Box<Arc<RwLock<DataInfo<Box<String>>>>> = data
        .storage_manager
        .get_data::<String>(vec!["heck"])
        .await
        .unwrap();
    let read2 = data.read().await;
    let read = read2.get_data().clone();
    drop(read2);
    ctx.say(format!("the uhh data: {}", read)).await?;
    Ok(())
}
