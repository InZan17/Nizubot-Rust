use chrono::{DateTime, TimeZone, Timelike, Utc};

use chrono_tz::Tz;
use poise::serenity_prelude::{Mentionable, User};

use crate::{Context, Error};

use super::time_format::TimeFormat;

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

    let time_format = ctx
        .data()
        .db
        .get_user_time_format(&ctx.author().id)
        .await
        .ok()
        .flatten()
        .unwrap_or(TimeFormat::TwentyFour);

    ctx.send(|m| m.content(format!("{}'s timezone is `{timezone_name}`.\nThe time for `{timezone_name}` is currently **{}**.", user.mention(), 
    get_time_string(date_time, time_format))).allowed_mentions(|m| {
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

    let time_format = ctx
        .data()
        .db
        .get_user_time_format(&ctx.author().id)
        .await
        .ok()
        .flatten()
        .unwrap_or(TimeFormat::TwentyFour);

    ctx.send(|m| {
        m.content(format!(
            "The time for `{}` is currently **{}**.",
            timezone.name(),
            get_time_string(date_time, time_format)
        ))
    })
    .await?;
    Ok(())
}

fn get_time_string(date_time: DateTime<Tz>, time_format: TimeFormat) -> String {
    match time_format {
        TimeFormat::Twelve => {
            let (pm, hour) = date_time.hour12();
            if pm {
                return format!("{}:{:0>2} PM", hour, date_time.minute());
            } else {
                return format!("{}:{:0>2} AM", hour, date_time.minute());
            }
        }
        TimeFormat::TwentyFour => {
            format!("{}:{:0>2}", date_time.hour(), date_time.minute())
        }
    }
}
