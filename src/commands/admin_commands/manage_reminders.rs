use std::{ops::Add, vec};

use crate::{
    managers::{detector_manager::DetectType, storage_manager::DataDirectories},
    utils::IdType,
    Context, Error,
};
use poise::serenity_prelude::{Member, Role, RoleId, User};

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
    #[description = "Which reminder do you wanna remove?"] index: u8,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id();

    let user_id = user.id;

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
        m.content(format!(
            "Successfully removed {}s reminder{}",
            user.name, message_ending
        ))
        .allowed_mentions(|a| a.empty_parse())
        .ephemeral(true)
    })
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

    let guild_id = ctx.guild_id();
    let user_id = user.id;

    let reminders = remind_manager.list_reminders(user_id, guild_id).await?;

    ctx.send(|m| {
        m.embed(|e| {
            e.title("Reminders")
                .description(format!("All of {}s reminders on this guild.", user.name))
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
        .ephemeral(true)
    })
    .await?;
    Ok(())
}
