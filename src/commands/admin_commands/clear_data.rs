use crate::{Context, Error};

/// Commands for messages.
#[poise::command(slash_command, default_member_permissions = "ADMINISTRATOR")]
pub async fn clear_data(ctx: Context<'_>) -> Result<(), Error> {
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
