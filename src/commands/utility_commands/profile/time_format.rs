use poise::{ChoiceParameter, CreateReply};
use serde::{Deserialize, Serialize};

use crate::{Context, Error};

#[derive(Serialize, Deserialize, Clone, Copy, ChoiceParameter)]
pub enum TimeFormat {
    #[name = "12-hour clock"]
    Twelve,
    #[name = "24-hour clock"]
    TwentyFour,
}

/// Command for setting your preferred time format.
#[poise::command(
    slash_command,
    subcommands("set", "remove"),
    subcommand_required,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn time_format(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Command to set your preferred time format.
#[poise::command(slash_command)]
pub async fn set(
    ctx: Context<'_>,
    #[description = "What's your preferred time format?"] time_format: TimeFormat,
) -> Result<(), Error> {
    let db = &ctx.data().db;

    let profile = ctx
        .data()
        .profile_manager
        .get_profile_data(ctx.author().id)
        .await;

    let mut profile_lock = profile.lock().await;

    let mut profile = profile_lock.get_profile(db).await?.clone();

    profile.time_format = Some(time_format);

    profile_lock.update_profile(profile, db).await?;

    ctx.send(
        CreateReply::default()
            .content(format!(
                "Sure! Your preferred time format has now been set to the {}.",
                time_format.name()
            ))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// Command to remove your preferred time format.
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

    profile.time_format = None;

    profile_lock.update_profile(profile, db).await?;

    ctx.send(
        CreateReply::default()
            .content("Your preferred time format has been removed!")
            .ephemeral(true),
    )
    .await?;
    Ok(())
}
