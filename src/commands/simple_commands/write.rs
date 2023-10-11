use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{managers::storage_manager::DataInfo, Context, Error};
use poise::serenity_prelude::RwLock;

/// Write!
#[poise::command(slash_command)]
pub async fn write(ctx: Context<'_>, #[description = "Write."] write: String) -> Result<(), Error> {
    let data = ctx.data();
    let data = data
        .storage_manager
        .get_data::<String>(vec!["heck"])
        .await
        .unwrap();
    let mut read2 = data.write().await;

    *read2.get_data_mut() = Box::new(write);

    drop(read2);

    ctx.say(format!("Written!")).await?;
    Ok(())
}
