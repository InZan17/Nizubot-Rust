use serde::{Deserialize, Serialize};

use crate::{Context, Error};

#[derive(Debug, Serialize, Deserialize)]
struct StoredData {
    pub content: String,
}

/// Read!
#[poise::command(slash_command)]
pub async fn read(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data();

    let opt: Option<StoredData> = data
        .db
        .query("SELECT * FROM stored_data:1")
        .await?
        .take(0)?;

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
    let data_json = serde_json::to_string(&data_struct)?;
    println!("{data_json}");

    data.db
        .query(format!("UPDATE stored_data:1 CONTENT {data_json};"))
        .await?;

    ctx.say(format!("Data written!")).await?;
    Ok(())
}
