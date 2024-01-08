use std::{
    ops::Add,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{managers::remind_manager::RemindInfo, Context, Error};

/// Command for reminders.
#[poise::command(
    slash_command,
    subcommands("add", "remove", "list"),
    subcommand_required
)]
pub async fn remind(ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Command to make me remind you of whatever you want.
#[poise::command(slash_command)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "When do you want me to remind you? Example: 1s 2m 3h 4d 5w 6y"]
    duration: String,
    #[description = "Message of the reminder."] message: Option<String>,
    #[description = "Should I put this reminder on a loop? (Default: False)"] looped: Option<bool>,
) -> Result<(), Error> {
    let looped = looped.unwrap_or(false);

    let Some(duration) = parse_duration_string(duration) else {
        ctx.send(|m| {
            m.content("Please give me a valid duration.")
                .ephemeral(true)
        })
        .await?;
        return Ok(());
    };

    if duration < 0. {
        ctx.send(|m| m.content("Duration cannot be negative.").ephemeral(true))
            .await?;
        return Ok(());
    }
    // 60 seconds = 1 minute, 60 minutes = 1 hour
    if looped && duration < 60. * 60. {
        ctx.send(|m| {
            m.content("When making a loop reminder, please make the duration 1 hour or longer.")
                .ephemeral(true)
        })
        .await?;
        return Ok(());
    }

    let message_ending;

    if let Some(message) = &message {
        message_ending = format!(" with: {message}")
    } else {
        message_ending = ".".to_string()
    }

    let guild_id;

    if let Some(id) = ctx.guild_id() {
        guild_id = Some(*id.as_u64());
    } else {
        guild_id = None;
    }

    let channel_id = *ctx.channel_id().as_u64();

    let add_result = ctx
        .data()
        .remind_manager
        .add_reminder(
            guild_id,
            channel_id,
            ctx.author().id,
            duration as u64,
            looped,
            message,
            || async {
                let remind_time = get_seconds() as f64 + duration;

                let handle;

                if looped {
                    handle = ctx
                        .send(|m| {
                            m.content(format!(
                                "Sure! I will now keep reminding you <t:{remind_time}:R>{message_ending}"
                            )).allowed_mentions(|a| {
                                a.empty_parse()
                            })
                        })
                        .await?;
                } else {
                    handle = ctx
                        .send(|m| {
                            m.content(format!(
                                "Sure! I will now remind you <t:{remind_time}:R>{message_ending}"
                            )).allowed_mentions(|a| {
                                a.empty_parse()
                            })
                        })
                        .await?;
                }

                let message_id = handle.message().await?.id.as_u64().clone();
                return Ok(message_id)
            }
        )
        .await;

    if let Err(err) = add_result {
        ctx.send(|m| {
            m.content(format!(
                "Sorry, I wasn't able to add that reminder. {}",
                err
            ))
            .ephemeral(true)
        })
        .await?;
        return Ok(());
    }

    Ok(())
}

/// Command to remove a reminder.
#[poise::command(slash_command)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "Which reminder to remove. (See reminders with /remind list)"] index: u8,
) -> Result<(), Error> {
    let guild_id;

    if let Some(id) = ctx.guild_id() {
        guild_id = Some(*id.as_u64());
    } else {
        guild_id = None;
    }

    let user_id = *ctx.author().id.as_u64();

    let removed_reminder = ctx
        .data()
        .remind_manager
        .remove_reminder(user_id, guild_id, index as usize)
        .await?;

    let Some(removed_reminder) = removed_reminder else {
        ctx.send(|m| {
            m.content("Failed to remove reminder. Are you using a valid index?")
                .ephemeral(true)
        })
        .await?;

        return Ok(());
    };

    let message_ending;

    if let Some(message) = removed_reminder.message {
        message_ending = format!(" <t:{}:R>: {}", removed_reminder.finish_time, message)
    } else {
        message_ending = format!(" <t:{}:R>.", removed_reminder.finish_time)
    }

    ctx.send(|m| {
        m.content(format!("Successfully removed reminder{}", message_ending))
            .allowed_mentions(|a| a.empty_parse())
    })
    .await?;

    Ok(())
}

/// Command to list reminders.
#[poise::command(slash_command)]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    let remind_manager = &ctx.data().remind_manager;
    let guild_id;
    if let Some(the_guild_id) = ctx.guild_id() {
        guild_id = Some(*the_guild_id.as_u64())
    } else {
        guild_id = None
    }

    let user_id = ctx.author().id.0;
    let reminders = remind_manager.list_reminders(user_id, guild_id).await?;

    ctx.send(|m| {
        m.embed(|e| {
            e.title("Reminders")
                .description("All of your reminders on this guild.")
                .footer(|f| f.text(format!("Total reminders: {}", reminders.len())));

            for (index, reminder) in reminders.iter().enumerate() {
                let mut ending = format!(" <#{}>", reminder.channel_id);

                if reminder.looping {
                    ending = ending.add(" (Looped)");
                }

                e.field(
                    format!("{index}: <t:{}:R>{ending}", reminder.finish_time),
                    reminder.message.clone().unwrap_or_default(),
                    false,
                );
            }

            e
        })
    })
    .await?;
    Ok(())
}

fn parse_duration_string(duration: String) -> Option<f64> {
    let mut total_duration = 0.0;
    for thing in duration.split(" ") {
        if thing.len() == 0 {
            continue;
        }
        let (amount, prefix) = thing.split_at(thing.len() - 1);

        if let Some(multiplier) = convert_prefix_to_multiplier(prefix.chars().next().unwrap()) {
            if let Ok(amount) = amount.parse::<f64>() {
                total_duration += amount * multiplier;
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
    Some(total_duration)
}

fn convert_prefix_to_multiplier(prefix: char) -> Option<f64> {
    match prefix {
        's' => Some(1.),
        'm' => Some(60.),
        'h' => Some(3600.),
        'd' => Some(86400.),
        'w' => Some(604800.),
        'y' => Some(31556926.),
        _ => None,
    }
}

fn get_seconds() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards. Oopsie.");

    since_the_epoch.as_secs()
}
