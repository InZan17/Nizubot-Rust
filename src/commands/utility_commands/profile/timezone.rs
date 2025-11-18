use chrono_tz::Tz;
use poise::CreateReply;

use crate::{Context, Error};

use crate::commands::utility_commands::check_timezone::autocomplete_timezone;

/// Command for setting and getting timezones.
#[poise::command(
    slash_command,
    subcommands("set", "remove"),
    subcommand_required,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn timezone(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Command to set your timezone.
#[poise::command(slash_command)]
pub async fn set(
    ctx: Context<'_>,
    #[max_length = 100]
    #[autocomplete = "autocomplete_timezone"]
    #[description = "What's your timezone?"]
    timezone: String,
) -> Result<(), Error> {
    let timezone = Tz::from_str_insensitive(&timezone)?;
    let tz_name = timezone.name();

    let db = &ctx.data().db;

    let profile = ctx
        .data()
        .profile_manager
        .get_profile_data(ctx.author().id)
        .await;

    let mut profile_lock = profile.lock().await;

    let mut profile = profile_lock.get_profile(db).await?.clone();

    profile.timezone = Some(tz_name.to_string());

    profile_lock.update_profile(profile, db).await?;

    ctx.send(
        CreateReply::default()
            .content(format!(
                "Sure! Your timezone has now been set to `{}`.",
                tz_name
            ))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// Command to remove your timezone.
#[poise::command(slash_command)]
pub async fn remove(ctx: Context<'_>) -> Result<(), Error> {
    let db = &ctx.data().db;

    let profile = ctx
        .data()
        .profile_manager
        .get_profile_data(ctx.author().id)
        .await;

    let mut profile_lock = profile.lock().await;

    let mut profile = profile_lock.get_profile(db).await?.clone();

    profile.timezone = None;

    profile_lock.update_profile(profile, db).await?;

    ctx.send(
        CreateReply::default()
            .content("Your timezone has been removed!")
            .ephemeral(true),
    )
    .await?;
    Ok(())
}
