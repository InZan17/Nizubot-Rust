use std::vec;

use crate::{
    managers::{detector_manager::DetectType, storage_manager::DataDirectories},
    utils::IdType,
    Context, Error,
};
use poise::serenity_prelude::{Role, RoleId};

/// Events for when bot detects a message.
#[poise::command(
    slash_command,
    subcommands("add", "remove", "list"),
    subcommand_required,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn detect_message(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Add event for when detecting a message.
#[poise::command(slash_command)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "How the detection will work."] detect_type: DetectType,
    #[description = "What I will detect."] key: String,
    #[description = "What I will respond with after detecting it."] response: String,
    #[description = "If my detection should be case-sensitive. (default: False)"]
    case_sensitive: Option<bool>,
) -> Result<(), Error> {
    let case_sensitive = case_sensitive.unwrap_or(false);

    let id;

    if let Some(guild_id) = ctx.guild_id() {
        id = IdType::GuildId(guild_id);
    } else {
        id = IdType::UserId(ctx.author().id);
    }

    let res = ctx
        .data()
        .detector_manager
        .add_message_detect(
            detect_type.clone(),
            key.clone(),
            response,
            case_sensitive,
            id,
        )
        .await;

    if let Err(err) = res {
        ctx.send(|m| {
            m.ephemeral(true).content(format!(
                "Sorry, I wasn't able to add that detector.\n\n{err}"
            ))
        })
        .await?;
        return Ok(());
    }

    ctx.send(|m| {
        m.ephemeral(true).content(format!(
            "Sure! I will now detect messages that {} \"{}\".",
            detect_type.to_sentence(),
            key
        ))
    })
    .await?;

    Ok(())
}

/// Remove event for when detecting a message.
#[poise::command(slash_command)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "Which detector you wanna remove."] index: u8,
) -> Result<(), Error> {
    let guild_or_user_id;
    let is_dms;

    if let Some(guild_id) = ctx.guild_id() {
        guild_or_user_id = *guild_id.as_u64();
        is_dms = false;
    } else {
        guild_or_user_id = *ctx.author().id.as_u64();
        is_dms = true;
    }

    let res = ctx
        .data()
        .detector_manager
        .remove_message_detect(index as usize, guild_or_user_id, is_dms)
        .await;

    if let Err(err) = res {
        ctx.send(|m| {
            m.ephemeral(true).content(format!(
                "Sorry, I wasn't able to delete that detector.\n\n{err}"
            ))
        })
        .await?;
        return Ok(());
    }

    ctx.send(|m| {
        m.ephemeral(true)
            .content("Sure! I have now removed that detection.")
    })
    .await?;

    Ok(())
}

/// List all message detectors in this guild.
#[poise::command(slash_command)]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    let id;

    if let Some(guild_id) = ctx.guild_id() {
        id = IdType::GuildId(guild_id);
    } else {
        id = IdType::UserId(ctx.author().id);
    }

    let detectors = ctx.data().detector_manager.get_message_detects(id).await?;

    ctx.send(|m| {
        m.embed(|e| {
            e.title("Message Detectors")
                .description("All of the message detectors in this guild.")
                .footer(|f| f.text(format!("Total detectors: {}", detectors.len())));

            for (index, detector) in detectors.iter().enumerate() {
                let ending;

                if detector.case_sensitive {
                    ending = " (case-sensitive)";
                } else {
                    ending = ""
                }

                e.field(
                    format!(
                        "{index}: {}: {}{ending}",
                        detector.detect_type.to_sentence(),
                        detector.key
                    ),
                    &detector.response,
                    false,
                );
            }

            e
        })
    })
    .await?;

    return Ok(());
}
