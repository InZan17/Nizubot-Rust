use chrono::{DateTime, TimeZone, Timelike, Utc};

use chrono_tz::{Tz, TZ_VARIANTS};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use poise::{
    serenity_prelude::{self},
    CreateReply,
};

use crate::{commands::utility_commands::profile::time_format::TimeFormat, Context, Error};

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

/// Command for checking the time in different timezones.
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn check_timezone(
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

    let db = &ctx.data().db;

    let profile = ctx
        .data()
        .profile_manager
        .get_profile_data(ctx.author().id)
        .await;

    let mut profile_lock = profile.lock().await;

    let profile_data = profile_lock.get_profile(db).await?;

    let time_format = profile_data.get_time_format_with_fallback(ctx.locale().unwrap());

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

pub fn get_time_string(date_time: DateTime<Tz>, time_format: TimeFormat) -> String {
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
