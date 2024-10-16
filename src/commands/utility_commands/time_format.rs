use poise::{
    serenity_prelude::{CreateAllowedMentions, Mentionable, User},
    ChoiceParameter, CreateReply,
};
use serde::{Deserialize, Serialize};

use crate::{Context, Error};

#[derive(Serialize, Deserialize, Clone, Copy, ChoiceParameter)]
pub enum TimeFormat {
    #[name = "12-hour clock"]
    Twelve,
    #[name = "24-hour clock"]
    TwentyFour,
}

/// Command for setting and getting users preferred time format.
#[poise::command(
    slash_command,
    subcommands("set", "remove", "get"),
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
    ctx.data()
        .db
        .set_user_time_format(&ctx.author().id, Some(time_format))
        .await?;

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
    ctx.data()
        .db
        .set_user_time_format(&ctx.author().id, None)
        .await?;

    ctx.send(
        CreateReply::default()
            .content("Your preferred time format has been removed!")
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

/// Command to check another user's preferred time format.
#[poise::command(slash_command)]
pub async fn get(
    ctx: Context<'_>,
    #[description = "Which user do you wanna check?"] user: Option<User>,
    #[description = "Should the message be hidden from others?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = ephemeral.unwrap_or(false);
    let user = user.as_ref().unwrap_or(ctx.author());

    let Some(time_format) = ctx.data().db.get_user_time_format(&ctx.author().id).await? else {
        ctx.send(
            CreateReply::default()
                .content("That user hasn't set their preferred time format to anything.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    ctx.send(
        CreateReply::default()
            .content(format!(
                "{}'s preferred time format is the {}.",
                user.mention(),
                time_format.name()
            ))
            .allowed_mentions(CreateAllowedMentions::new())
            .ephemeral(ephemeral),
    )
    .await?;
    Ok(())
}
