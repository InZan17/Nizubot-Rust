use std::vec;

use crate::{managers::detector_manager::DetectType, utils::IdType, Context, Error};
use poise::{
    serenity_prelude::{CreateEmbed, CreateEmbedFooter},
    CreateReply,
};

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
#[poise::command(
    slash_command,
    required_bot_permissions = "SEND_MESSAGES | VIEW_CHANNEL"
)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "How should the detection happen?"] detect_type: DetectType,
    #[max_length = 50]
    #[description = "What should I detect?"]
    key: String,
    #[max_length = 500]
    #[description = "What will I respond with after detecting it?"]
    response: String,
    #[description = "Should my detection be case-sensitive? (Default: False)"]
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
            response.clone(),
            case_sensitive,
            id,
        )
        .await;

    if let Err(err) = res {
        ctx.send(
            CreateReply::default()
                .content(format!(
                    "Sorry, I wasn't able to add that detector.\n\n{}",
                    err.to_string()
                ))
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    ctx.send(CreateReply::default().content(format!(
        "Sure! I will now detect messages that {} \"{}\" and I will reply with \"{}\".",
        detect_type.to_sentence(),
        key,
        response
    )))
    .await?;

    Ok(())
}

/// Remove event for when detecting a message.
#[poise::command(slash_command)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "Which detector do you want me to remove?"] index: u8,
) -> Result<(), Error> {
    let id;

    if let Some(guild_id) = ctx.guild_id() {
        id = IdType::GuildId(guild_id);
    } else {
        id = IdType::UserId(ctx.author().id);
    }

    let res = ctx
        .data()
        .detector_manager
        .remove_message_detect(index as usize, id)
        .await;

    if let Err(err) = res {
        ctx.send(
            CreateReply::default()
                .content(format!(
                    "Sorry, I wasn't able to delete that detector.\n\n{}",
                    err.to_string()
                ))
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    ctx.send(CreateReply::default().content("Sure! I have now removed that detection."))
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

    let data = ctx.data();
    let db = &data.db;

    let detector_data = ctx.data().detector_manager.get_detectors_data(id).await;
    let mut locked_detector_data = detector_data.lock().await;
    let detectors_result = locked_detector_data.get_detectors(&db).await;

    let detectors = match detectors_result {
        Ok(ok) => ok,
        Err(err) => {
            ctx.send(
                CreateReply::default()
                    .content(format!(
                        "Sorry, I wasn't able to list the detectors.\n\n{}",
                        err.to_string()
                    ))
                    .ephemeral(true),
            )
            .await?;
            return Ok(());
        }
    };

    let mut create_embed = CreateEmbed::new()
        .title("Message Detectors")
        .description("All of the message detectors in this guild.")
        .footer(CreateEmbedFooter::new(format!(
            "Total detectors: {}",
            detectors.len()
        )));

    for (index, detector) in detectors.iter().enumerate() {
        let ending;

        if detector.case_sensitive {
            ending = " (case-sensitive)";
        } else {
            ending = ""
        }

        create_embed = create_embed.field(
            format!(
                "{index}: {}: {}{ending}",
                detector.detect_type.to_sentence(),
                detector.key
            ),
            &detector.response,
            false,
        );
    }

    ctx.send(CreateReply::default().embed(create_embed)).await?;

    return Ok(());
}
