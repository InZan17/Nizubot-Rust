use crate::{
    managers::log_manager::{LogManager, LogSource, LogType},
    utils::IdType,
    Context, Error,
};
use poise::{serenity_prelude::CreateAttachment, CreateReply};

/// Logs for debugging.
#[poise::command(
    slash_command,
    subcommands("get", "add", "clear"),
    subcommand_required,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn log(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Gets user log in dms and guild log in guilds.
#[poise::command(slash_command)]
pub async fn get(ctx: Context<'_>) -> Result<(), Error> {
    let id;

    if let Some(guild_id) = ctx.guild_id() {
        id = IdType::GuildId(guild_id)
    } else {
        id = IdType::UserId(ctx.author().id)
    }

    let logs = ctx.data().log_manager.get_logs(&id).await?;

    ctx.send(CreateReply::default().attachment(CreateAttachment::bytes(
        logs.as_bytes(),
        LogManager::get_file_name(&id),
    )))
    .await?;

    Ok(())
}

/// Adds to the guild/user log.
#[poise::command(slash_command)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "What should I add to the log?"] message: String,
) -> Result<(), Error> {
    let id;
    let log_type;

    if let Some(guild_id) = ctx.guild_id() {
        id = IdType::GuildId(guild_id);
        log_type = "guild";
    } else {
        id = IdType::UserId(ctx.author().id);
        log_type = "user";
    }

    ctx.data()
        .log_manager
        .add_log(
            &id,
            message,
            LogType::Info,
            LogSource::Custom(ctx.author().id.to_string()),
        )
        .await?;
    ctx.say(format!("Added to {log_type} logs!")).await?;
    Ok(())
}

/// Clears the guild/user log.
#[poise::command(slash_command)]
pub async fn clear(ctx: Context<'_>) -> Result<(), Error> {
    let id;

    if let Some(guild_id) = ctx.guild_id() {
        id = IdType::GuildId(guild_id)
    } else {
        id = IdType::UserId(ctx.author().id)
    }

    ctx.data().log_manager.clear_log(&id).await?;
    ctx.say("Cleared!").await?;
    return Ok(());
}
