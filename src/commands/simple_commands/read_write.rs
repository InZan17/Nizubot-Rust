use serde::{Deserialize, Serialize};

use crate::{managers::db::StoredData, Context, Error};

/// Read!
#[poise::command(slash_command)]
pub async fn read(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data();

    let opt = data.db.get_single_data().await?;

    let content = if let Some(stored_data) = opt {
        stored_data.content
    } else {
        "".to_string()
    };

    ctx.send(|m| {
        m.content(format!("Written data: {}", content))
            .allowed_mentions(|a| a.empty_parse())
    })
    .await?;
    Ok(())
}

/// Writes data!
#[poise::command(slash_command)]
pub async fn write(ctx: Context<'_>, #[description = "Write."] write: String) -> Result<(), Error> {
    let data = ctx.data();

    let data_struct = StoredData { content: write };

    data.db.update_single_data(&data_struct).await?;

    ctx.say(format!("Data written!")).await?;
    Ok(())
}
