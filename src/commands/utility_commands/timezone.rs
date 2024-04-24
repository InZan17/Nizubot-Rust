use chrono::{TimeZone, Timelike, Utc};

use chrono_tz::Tz;
use poise::serenity_prelude::{Mentionable, User};

use crate::{Context, Error};

/// Command for setting and getting timezones.
#[poise::command(
    slash_command,
    subcommands("set", "remove", "check", "user"),
    subcommand_required
)]
pub async fn timezone(ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Command to set your timezone.
#[poise::command(slash_command)]
pub async fn set(
    ctx: Context<'_>,
    #[max_length = 100]
    #[description = "What's your timezone?"]
    timezone: String,
) -> Result<(), Error> {
    let timezone = Tz::from_str_insensitive(&timezone)?;
    let tz_name = timezone.name();
    ctx.data()
        .db
        .set_user_timezone(&ctx.author().id, Some(tz_name.to_string()))
        .await?;

    ctx.send(|m| {
        m.content(format!(
            "Sure! Your timezone has now been set to `{}`.",
            tz_name
        ))
    })
    .await?;

    Ok(())
}

/// Command to remove your timezone.
#[poise::command(slash_command)]
pub async fn remove(ctx: Context<'_>) -> Result<(), Error> {
    ctx.data()
        .db
        .set_user_timezone(&ctx.author().id, None)
        .await?;

    ctx.send(|m| m.content("Your timezone has been removed!"))
        .await?;
    Ok(())
}

/// Command to check another user's timezone and time.
#[poise::command(slash_command)]
pub async fn user(
    ctx: Context<'_>,
    #[description = "Which user do you wanna check?"] user: Option<User>,
) -> Result<(), Error> {
    let user = user.as_ref().unwrap_or(ctx.author());
    let Some(timezone_name) = ctx.data().db.get_user_timezone(&user.id).await? else {
        ctx.send(|m| {
            m.content("That user hasn't set their timezone to anything.")
                .ephemeral(true)
        })
        .await?;
        return Ok(());
    };
    let now = Utc::now();
    let timezone = Tz::from_str_insensitive(&timezone_name)?;
    let date_time = timezone.from_utc_datetime(&now.naive_utc());
    ctx.send(|m| m.content(format!("{}'s timezone is `{timezone_name}`.\nThe time for `{timezone_name}` is currently **{}:{:0>2}**.", user.mention(), date_time.hour(), date_time.minute())).allowed_mentions(|m| {
        m.empty_parse()
    }))
        .await?;
    Ok(())
}

/// Command to check the time of a specific timezone.
#[poise::command(slash_command)]
pub async fn check(
    ctx: Context<'_>,
    #[description = "Which timezone do you wanna check?"] timezone: String,
) -> Result<(), Error> {
    let timezone = Tz::from_str_insensitive(&timezone)?;
    let now = Utc::now();
    let date_time = timezone.from_utc_datetime(&now.naive_utc());
    ctx.send(|m| {
        m.content(format!(
            "The time for `{}` is currently **{}:{:0>2}**.",
            timezone.name(),
            date_time.hour(),
            date_time.minute()
        ))
    })
    .await?;
    Ok(())
}
