use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Context, Error};

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
            m.content("Please give me a valid duration.").ephemeral(true)
        }).await?;
        return Ok(())
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
    let channel_id;

    if let Some(id) = ctx.guild_id() {
        guild_id = Some(*id.as_u64());
        channel_id = Some(*ctx.channel_id().as_u64());
    } else {
        guild_id = None;
        channel_id = None;
    }

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
        )
        .await;

    match add_result {
        Err(err) => {
            ctx.send(|m| {
                m.content(format!(
                    "Sorry, I wasn't able to add that reminder. {}",
                    err
                )).ephemeral(true)
            })
            .await?;
            return Ok(());
        },

        Ok((index, data_holder)) => {
            let remind_time = get_seconds() as f64 + duration;

            let handle;

            if looped {
                handle = ctx
                    .send(|m| {
                        m.content(format!(
                            "Sure! I will now keep reminding you <t:{remind_time}:R>{message_ending}"
                        ))
                    })
                    .await?;
            } else {
                handle = ctx
                    .send(|m| {
                        m.content(format!(
                            "Sure! I will now remind you <t:{remind_time}:R>{message_ending}"
                        ))
                    })
                    .await?;
            }

            let message_id = handle.message().await?.id.as_u64().clone();

            let mut data_mut = data_holder.get_data_mut().await;
            let remind_info = &mut data_mut[index];

            remind_info.message_id = Some(message_id);

            drop(data_mut);

            data_holder.request_file_write().await;

        }
    }
    Ok(())
}

/// Command to remove a reminder.
#[poise::command(slash_command)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "Which reminder to remove. (See reminders with /remind list)"] index: u8,
) -> Result<(), Error> {
    let a: bool = todo!();
    let removed: bool = todo!();

    if !a {
        ctx.send(|m| {
            m.content("Failed to remove reminder. Are you using a valid index?")
                .ephemeral(true)
        })
        .await?;
        return Ok(());
    }

    let message_ending;

    if !removed {
        message_ending = ".".to_string()
    } else {
        if true {
            //removed.message
            message_ending = format!(" <t:{}:R>.", "Finisejd time")
        } else {
            message_ending = format!(" <t:{}:R>: {}", "Finisejd time", "remoed.messafg")
        }
    }

    ctx.send(|m| m.content(format!("Successfully removed reminder{}", message_ending)))
        .await?;

    Ok(())
}

/// Command to list reminders.
#[poise::command(slash_command)]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    /*
    local reminders = _G.reminder:listReminders(ia.guildId, ia.channelId, ia.user.id)
    local embed = {
        title = "Reminders",
        description = "All of your reminders on this channel.",
        fields = {},
        footer = {
            text = "Total reminders: "..#reminders
        }
    }

    for i, v in ipairs(reminders) do
        local ending = ""
        if v.looping then
            ending = " (Looped)"
        end
        table.insert(embed.fields, {
            name = i..": <t:"..v.finishedTime..":R>"..ending,
            value = v.message
        })
    end
    ia:reply{embed=embed}
    */
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
