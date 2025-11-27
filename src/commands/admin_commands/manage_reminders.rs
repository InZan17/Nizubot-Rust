use std::{ops::Add, vec};

use crate::{
    commands::utility_commands::check_timezone::get_time_string,
    managers::profile_manager::locale_time_format, Context, Error,
};
use chrono::{DateTime, Datelike, TimeZone};
use chrono_tz::Tz;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use poise::{
    serenity_prelude::{
        self, CreateAllowedMentions, CreateEmbed, CreateEmbedFooter, Unresolved, User,
    },
    CreateReply,
};

async fn autocomplete_reminder_index(
    ctx: Context<'_>,
    partial: &str,
) -> Vec<poise::serenity_prelude::AutocompleteChoice> {
    let poise::Context::Application(application_context) = ctx else {
        return vec![];
    };

    let Some(poise::serenity_prelude::ResolvedValue::Unresolved(Unresolved::User(user_id))) =
        application_context
            .args
            .iter()
            .find(|arg| arg.name == "user")
            .map(|arg| arg.value.clone())
    else {
        return vec![];
    };

    let reminders_data = ctx.data().remind_manager.get_reminders_data(user_id).await;

    let mut locked_reminders_data = reminders_data.lock().await;

    let Ok(reminders) = locked_reminders_data.get_reminders(&ctx.data().db).await else {
        return vec![];
    };

    let (timezone, time_format) = {
        let profile_data = ctx.data().profile_manager.get_profile_data(user_id).await;

        let mut profile_lock = profile_data.lock().await;

        match profile_lock.get_profile(&ctx.data().db).await {
            Ok(profile) => {
                let timezone = profile
                    .get_timezone()
                    .and_then(|(_, tz)| tz)
                    .unwrap_or(Tz::UTC);
                let time_format = profile.get_time_format_with_fallback(ctx.locale().unwrap());
                (timezone, time_format)
            }
            Err(_) => (Tz::UTC, locale_time_format(ctx.locale().unwrap())),
        }
    };

    let matcher = SkimMatcherV2::default().ignore_case();

    let mut reminder_names = reminders
        .iter()
        .enumerate()
        .rev()
        .map(|(index, value)| {
            let date = DateTime::from_timestamp(value.finish_time as i64, 0)
                .map(|date| timezone.from_utc_datetime(&date.naive_utc()))
                .map(|date| {
                    let time = get_time_string(date, time_format);
                    format!(
                        "{}-{}-{} {} ({})",
                        date.year(),
                        date.month(),
                        date.day(),
                        time,
                        timezone.name()
                    )
                })
                .unwrap_or_else(|| "(broken date)".to_string());

            let message = if let Some(mut message) = value.message.clone() {
                if message.len() > 20 {
                    message = message.chars().take(30).collect::<String>();
                    message = message.trim_end().to_string();
                    message.push_str("...");
                }
                format!("-> {message}")
            } else {
                String::new()
            };
            (
                format!(
                    "{index}: {date}{}{}",
                    if value.looping { " (looped) " } else { " " },
                    message
                ),
                index,
            )
        })
        .collect::<Vec<_>>();

    reminder_names.retain(|(key, _)| matcher.fuzzy_match(key, partial).is_some());

    // calling fuzzy_match again for a second time is fine cause it does caching
    reminder_names.sort_by_key(|(key, _)| matcher.fuzzy_match(key, partial).unwrap_or(-1));

    reminder_names
        .into_iter()
        .rev() // Reverse because higher score is better.
        .map(|(key, index)| serenity_prelude::AutocompleteChoice::new(key.to_string(), index))
        .collect()
}

/// Command to check and remove other users reminders.
#[poise::command(
    slash_command,
    subcommands("peek", "remove"),
    subcommand_required,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn manage_reminders(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Command to remove a members reminder.
#[poise::command(slash_command)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "Which user owns the reminder?"] user: User,
    #[autocomplete = "autocomplete_reminder_index"]
    #[description = "Which reminder do you wanna remove?"]
    index: u8,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id();

    let user_id = user.id;

    let removed_reminder = ctx
        .data()
        .remind_manager
        .remove_reminder(user_id, guild_id, index as usize)
        .await?;

    let message_ending;

    if let Some(message) = removed_reminder.message {
        message_ending = format!(" <t:{}:R>: {}", removed_reminder.finish_time, message)
    } else {
        message_ending = format!(" <t:{}:R>.", removed_reminder.finish_time)
    }

    ctx.send(
        CreateReply::default()
            .content(format!(
                "Successfully removed {}s reminder{}",
                user.name, message_ending
            ))
            .allowed_mentions(CreateAllowedMentions::new())
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// Command to list reminders of a member.
#[poise::command(slash_command)]
pub async fn peek(
    ctx: Context<'_>,
    #[description = "Which user to check the reminders for?"] user: User,
) -> Result<(), Error> {
    let remind_manager = &ctx.data().remind_manager;
    let db = &ctx.data().db;

    let guild_id = ctx.guild_id();
    let user_id = user.id;

    let reminders_data = remind_manager.get_reminders_data(user_id).await;
    let mut locked_reminders_data = reminders_data.lock().await;

    let user_reminders = locked_reminders_data.get_reminders(&db).await?;

    let filtered_reminders = user_reminders
        .iter()
        .filter(|remind_info| remind_info.guild_id == guild_id)
        .collect::<Vec<_>>();

    let mut create_embed = CreateEmbed::new()
        .title("Reminders")
        .description(format!("All of {}s reminders on this guild.", user.name))
        .footer(CreateEmbedFooter::new(format!(
            "Total reminders: {}",
            filtered_reminders.len()
        )));

    for (index, reminder) in filtered_reminders.into_iter().enumerate() {
        let mut ending = format!(" <#{}>", reminder.channel_id);

        if reminder.looping {
            ending = ending.add(" (Looped)");
        }

        create_embed = create_embed.field(
            format!("{index}: <t:{}:R>{ending}", reminder.finish_time),
            reminder.message.clone().unwrap_or_default(),
            false,
        );
    }

    ctx.send(CreateReply::default().embed(create_embed).ephemeral(true))
        .await?;
    Ok(())
}
