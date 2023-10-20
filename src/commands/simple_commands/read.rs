use crate::{Context, Error};

/// Read!
#[poise::command(slash_command)]
pub async fn read(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data();
    let data = data
        .storage_manager
        .get_data_or_default::<String>(vec!["storing"], "Nothing".to_owned())
        .await;

    ctx.say(format!("Written data: {}", *data.get_data().await))
        .await?;
    Ok(())
}
