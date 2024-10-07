use poise::{serenity_prelude::CreateAllowedMentions, CreateReply};

use crate::{managers::db::StoredData, Context, Error};

/// Reads data!
#[poise::command(slash_command)]
pub async fn read(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data();

    let opt = data.db.get_single_data().await?;

    let content = if let Some(stored_data) = opt {
        stored_data.content
    } else {
        "".to_string()
    };

    ctx.send(
        CreateReply::default()
            .content(format!("Written data: {}", content))
            .allowed_mentions(CreateAllowedMentions::new()),
    )
    .await?;
    Ok(())
}

/// Writes data!
#[poise::command(slash_command)]
pub async fn write(
    ctx: Context<'_>,
    #[max_length = 500]
    #[description = "Write."]
    write: String,
) -> Result<(), Error> {
    let data = ctx.data();

    let data_struct = StoredData { content: write };

    data.db.update_single_data(&data_struct).await?;

    ctx.say(format!("Data written!")).await?;
    Ok(())
}
