use chrono::{DateTime, TimeZone, Timelike, Utc};

use chrono_tz::{Tz, TZ_VARIANTS};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use poise::{
    serenity_prelude::{self, CreateAllowedMentions, Mentionable, User},
    CreateReply,
};

use crate::{Context, Error};

use super::time_format::TimeFormat;

pub async fn autocomplete_timezone(
    _: Context<'_>,
    partial: &str,
) -> Vec<poise::serenity_prelude::AutocompleteChoice> {
    let matcher = SkimMatcherV2::default().ignore_case();

    let mut valid_timezones = TZ_VARIANTS
        .into_iter()
        .filter(|tz| matcher.fuzzy_match(tz.name(), partial).is_some())
        .collect::<Vec<_>>();

    valid_timezones.sort_by_key(|key| matcher.fuzzy_match(key.name(), partial).unwrap_or(-1));

    valid_timezones
        .into_iter()
        .rev() // Reverse because higher score is better.
        .map(|tz| serenity_prelude::AutocompleteChoice::new(tz.name(), tz.name()))
        .collect()
}

/// Command for setting and getting timezones.
#[poise::command(
    slash_command,
    subcommands("set", "remove", "check", "user"),
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
    ctx.data()
        .db
        .set_user_timezone(&ctx.author().id, Some(tz_name.to_string()))
        .await?;

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
    ctx.data()
        .db
        .set_user_timezone(&ctx.author().id, None)
        .await?;

    ctx.send(
        CreateReply::default()
            .content("Your timezone has been removed!")
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

/// Command to check another user's timezone and time.
#[poise::command(slash_command)]
pub async fn user(
    ctx: Context<'_>,
    #[description = "Which user do you wanna check?"] user: Option<User>,
    #[description = "Should the message be hidden from others?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = ephemeral.unwrap_or(false);
    let user = user.as_ref().unwrap_or(ctx.author());
    let Some(timezone_name) = ctx.data().db.get_user_timezone(&user.id).await? else {
        ctx.send(
            CreateReply::default()
                .content("That user hasn't set their timezone to anything.")
                .ephemeral(true),
        )
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

    ctx.send(CreateReply::default().content(format!("{}'s timezone is `{timezone_name}`.\nThe time for `{timezone_name}` is currently **{}**.", user.mention(), 
    get_time_string(date_time, time_format))).allowed_mentions(CreateAllowedMentions::new())
    .ephemeral(ephemeral))
        .await?;
    Ok(())
}

/// Command to check the time of a specific timezone.
#[poise::command(slash_command)]
pub async fn check(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_timezone"]
    #[description = "Which timezone do you wanna check?"]
    timezone: String,
    #[description = "Should the message be hidden from others?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = ephemeral.unwrap_or(false);
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

    ctx.send(
        CreateReply::default()
            .content(format!(
                "The time for `{}` is currently **{}**.",
                timezone.name(),
                get_time_string(date_time, time_format)
            ))
            .ephemeral(ephemeral),
    )
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
