use crate::{Context, Error};

/// Writes data!
#[poise::command(slash_command)]
pub async fn write(ctx: Context<'_>, #[description = "Write."] write: String) -> Result<(), Error> {
    let data = ctx.data();
    let data = data
        .storage_manager
        .get_data_or_default::<String>(vec!["storing"], "".to_string())
        .await;

    *data.get_data_mut().await = write;

    data.request_file_write().await;

    ctx.say(format!("Data written!")).await?;
    Ok(())
}
