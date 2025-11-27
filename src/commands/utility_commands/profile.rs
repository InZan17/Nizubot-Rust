use chrono::{TimeZone, Utc};
use poise::{
    serenity_prelude::{CreateEmbed, User},
    ChoiceParameter, CreateReply,
};

use crate::{commands::utility_commands::check_timezone::get_time_string, Context, Error};

pub mod time_format;
mod timezone;

use time_format::time_format;
use timezone::timezone;

/// Command for checking/changing your profile.
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel",
    subcommands("check", "clear", "timezone", "time_format"),
    subcommand_required
)]
pub async fn profile(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Check your/someone elses profile.
#[poise::command(
    slash_command,
    required_bot_permissions = "VIEW_CHANNEL | SEND_MESSAGES | READ_MESSAGE_HISTORY"
)]
pub async fn check(
    ctx: Context<'_>,
    #[description = "Which user do you wanna check the profile for? (Default: You)"] user: Option<
        User,
    >,
    #[description = "Should the message be hidden from others? (Default: False)"] ephemeral: Option<
        bool,
    >,
) -> Result<(), Error> {
    let ephemeral = ephemeral.unwrap_or(false);
    let user = user.as_ref().unwrap_or(ctx.author());

    let db = &ctx.data().db;

    let profile = ctx.data().profile_manager.get_profile_data(user.id).await;

    let mut profile_lock = profile.lock().await;

    let profile_data = profile_lock.get_profile(db).await?;

    let preferred_time_format = match profile_data.time_format {
        Some(time_format) => time_format.name(),
        None => "None selected",
    };

    let profile_timezone = profile_data.get_timezone();

    let caller_time_format = if user.id == ctx.author().id {
        profile_data.get_time_format_with_fallback(ctx.locale().unwrap())
    } else {
        drop(profile_lock);
        let profile = ctx
            .data()
            .profile_manager
            .get_profile_data(ctx.author().id)
            .await;

        let mut profile_lock = profile.lock().await;

        let profile_data = profile_lock.get_profile(db).await?;
        profile_data.get_time_format_with_fallback(ctx.locale().unwrap())
    };

    let timezone = match profile_timezone {
        Some((timezone, Some(tz))) => {
            let now = Utc::now();
            let date_time = tz.from_utc_datetime(&now.naive_utc());
            format!(
                "{timezone}: ({})",
                get_time_string(date_time, caller_time_format)
            )
        }
        Some((timezone, None)) => timezone,
        None => "None selected".to_string(),
    };

    let embed = CreateEmbed::new()
        .title(format!("{}'s profile", user.name))
        .thumbnail(
            user.avatar_url()
                .unwrap_or_else(|| user.default_avatar_url()),
        )
        .field("Preferred time format", preferred_time_format, false)
        .field("Timezone", timezone, false);

    ctx.send(CreateReply::default().embed(embed).ephemeral(ephemeral))
        .await?;
    Ok(())
}

/// Clears your profile.
#[poise::command(
    slash_command,
    required_bot_permissions = "VIEW_CHANNEL | SEND_MESSAGES | READ_MESSAGE_HISTORY"
)]
pub async fn clear(
    ctx: Context<'_>,
    #[description = "Are you sure you want to clear your profile?"] confirmation: Option<bool>,
) -> Result<(), Error> {
    let confirmation = confirmation.unwrap_or(false);

    if !confirmation {
        ctx.reply("Are you sure you wanna clear your profile? Set the `confirmation` parameter to `True` to confirm.")
            .await?;
        return Ok(());
    }

    let db = &ctx.data().db;

    let profile = ctx
        .data()
        .profile_manager
        .get_profile_data(ctx.author().id)
        .await;

    let mut profile_lock = profile.lock().await;

    profile_lock.delete_profile(db).await?;

    ctx.send(
        CreateReply::default()
            .content("Successfully cleared your profile.")
            .ephemeral(true),
    )
    .await?;
    Ok(())
}
