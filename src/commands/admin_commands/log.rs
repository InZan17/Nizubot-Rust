use crate::{
    managers::log_manager::{LogManager, LogSource, LogType},
    utils::IdType,
    Context, Error,
};
use poise::{
    serenity_prelude::{CreateAttachment, Webhook},
    CreateReply,
};

/// Logs for debugging.
#[poise::command(
    slash_command,
    subcommands("get", "add", "add_webhook", "remove_webhook"),
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

    let logs = ctx.data().log_manager.get_formatted(id).await;

    ctx.send(
        CreateReply::default()
            .content("Showing logs from the last 12 hours.")
            .attachment(CreateAttachment::bytes(
                logs.as_bytes(),
                LogManager::get_file_name(id),
            )),
    )
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
            id,
            message,
            LogType::Info,
            LogSource::Custom(ctx.author().id.to_string()),
        )
        .await;
    ctx.say(format!("Added to {log_type} logs!")).await?;
    Ok(())
}

/// Makes the bot use a webhook to send logs.
#[poise::command(slash_command)]
pub async fn add_webhook(
    ctx: Context<'_>,
    #[description = "Do you have an existing webhook you want me to use?"] webhook_url: Option<
        String,
    >,
) -> Result<(), Error> {
    let id;
    let log_type;
    let webhook_or_channel;

    let bot_id = ctx.framework().bot_id;

    if let Some((guild_id, bot_member_name)) = ctx.guild().map(|guild| {
        (
            guild.id,
            guild
                .members
                .get(&bot_id)
                .map(|member| member.display_name().to_string()),
        )
    }) {
        id = IdType::GuildId(guild_id);
        log_type = "guild";

        webhook_or_channel = match webhook_url {
            Some(webhook_url) => {
                let webhook = Webhook::from_url(&ctx, &webhook_url).await?;
                either::Either::Left(webhook)
            }
            None => {
                let bot_name = match bot_member_name {
                    Some(bot_name) => bot_name,
                    None => ctx
                        .http()
                        .get_member(guild_id, bot_id)
                        .await?
                        .display_name()
                        .to_string(),
                };
                either::Either::Right((ctx.channel_id(), bot_name))
            }
        };
    } else {
        id = IdType::UserId(ctx.author().id);
        log_type = "user";

        let Some(webhook_url) = webhook_url else {
            return Err("To set a webhook to user logs, you need to provide one yourself.".into());
        };

        let webhook = Webhook::from_url(&ctx, &webhook_url).await?;
        webhook_or_channel = either::Either::Left(webhook);
    }

    ctx.data()
        .log_manager
        .add_webhook(id, webhook_or_channel)
        .await?;
    ctx.say(format!("Successfully added webhook to {log_type} logs!"))
        .await?;
    Ok(())
}

/// Makes the bot no longer use a webhook to send logs.
#[poise::command(slash_command)]
pub async fn remove_webhook(ctx: Context<'_>) -> Result<(), Error> {
    let id;
    let log_type;

    if let Some(guild_id) = ctx.guild_id() {
        id = IdType::GuildId(guild_id);
        log_type = "guild";
    } else {
        id = IdType::UserId(ctx.author().id);
        log_type = "user";
    }

    ctx.data().log_manager.remove_webhook(id).await?;
    ctx.say(format!(
        "Successfully removed webhook from {log_type} logs!"
    ))
    .await?;
    Ok(())
}
