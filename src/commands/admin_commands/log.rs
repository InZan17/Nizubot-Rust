use crate::{Context, Error};
use poise::serenity_prelude::{AttachmentType, Emoji, User};

//TODO make these administrator only

/// Logs for debugging.
#[poise::command(slash_command, subcommands("get", "add", "clear"), subcommand_required)]
pub async fn log(ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Gets user log in dms and server log in server.
#[poise::command(slash_command)]
pub async fn get(ctx: Context<'_>) -> Result<(), Error> {
    let logs = ctx
        .data()
        .log_manager
        .get_user_logs(&ctx.author().id)
        .await?;

    ctx.send(|m| {
        m.attachment(AttachmentType::Bytes {
            data: std::borrow::Cow::Borrowed(logs.as_bytes()),
            filename: "logs.txt".to_string(),
        })
    })
    .await?;

    Ok(())
}

/// Adds to the server/user log.
#[poise::command(slash_command)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "What to add."] add: String,
) -> Result<(), Error> {
    ctx.data()
        .log_manager
        .add_user_log(&ctx.author().id, add)
        .await?;
    ctx.say("Done!").await?;
    Ok(())
}

/// Clears the server/user log.
#[poise::command(slash_command)]
pub async fn clear(ctx: Context<'_>) -> Result<(), Error> {
    ctx.data()
        .log_manager
        .clear_user_log(&ctx.author().id)
        .await?;
    ctx.say("Cleared!").await?;
    return Ok(());
}
