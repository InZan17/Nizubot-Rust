use serde::{Deserialize, Serialize};

use crate::{
    managers::db::{IsConnected, Record},
    Context, Error,
};

#[derive(Debug, Serialize, Deserialize)]
struct StoredData {
    pub content: String,
}

/// Read!
#[poise::command(slash_command)]
pub async fn read(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data();

    data.db.is_connected().await?;

    let opt: Option<StoredData> = data.db.select(("stored_data", 1)).await?;

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

    data.db.is_connected().await?;

    data.db
        .update::<Option<Record>>(("stored_data", 1))
        .content(StoredData { content: write })
        .await?;

    ctx.say(format!("Data written!")).await?;
    Ok(())
}
