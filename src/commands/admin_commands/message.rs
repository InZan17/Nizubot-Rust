use poise::{
    serenity_prelude::{
        CreateAttachment, CreateEmbed, CreateMessage, EditMessage, Embed, Message, MessageType,
    },
    CreateReply,
};

use crate::{Context, Error};

/// Commands for messages.
#[poise::command(
    slash_command,
    subcommands("edit", "send", "analyze"),
    subcommand_required,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn message(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Get information about a message.
#[poise::command(
    slash_command,
    required_bot_permissions = "READ_MESSAGE_HISTORY | VIEW_CHANNEL"
)]
pub async fn analyze(
    ctx: Context<'_>,
    #[description = "The message you want to get information from."] message_id: Message,
) -> Result<(), Error> {
    let data = serde_json::to_string_pretty(&message_id).unwrap();
    let data_bytes = data.as_bytes();
    ctx.send(CreateReply::default().attachment(CreateAttachment::bytes(data_bytes, "message.txt")))
        .await?;

    Ok(())
}

/// I will say what you want but not show that you ran a command.
#[poise::command(
    slash_command,
    required_bot_permissions = "SEND_MESSAGES | VIEW_CHANNEL"
)]
pub async fn send(
    ctx: Context<'_>,
    #[max_length = 2000]
    #[description = "Contents of the message."]
    content: Option<String>,
    #[description = "Embeds of the message."] embeds: Option<String>,
) -> Result<(), Error> {
    let empty_workaround = content.is_none() && embeds.is_none();

    let mut single_embed = None;
    let mut multiple_embeds = None;

    if let Some(embeds) = embeds {
        single_embed = serde_json::from_str::<Embed>(&embeds).ok();
        if single_embed.is_none() {
            multiple_embeds = serde_json::from_str::<Vec<Embed>>(&embeds).ok();
            if multiple_embeds.is_none() {
                ctx.send(
                    CreateReply::default()
                        .content("Please send valid embed data.")
                        .ephemeral(true),
                )
                .await?;
            }
        }
    }

    let mut create_message = CreateMessage::new();

    if let Some(embed) = single_embed {
        create_message = create_message.embed(CreateEmbed::from(embed));
    } else if let Some(embeds) = multiple_embeds {
        for embed in embeds {
            create_message = create_message.embed(CreateEmbed::from(embed));
        }
    }
    if let Some(content) = content {
        create_message = create_message.content(content);
    } else if empty_workaround {
        create_message = create_message.content("**\n**");
    }

    let message_result = ctx.channel_id().send_message(ctx, create_message).await;

    if let Err(err) = message_result {
        ctx.send(
            CreateReply::default()
                .content(format!("An error has occured:\n{}", err))
                .ephemeral(true),
        )
        .await?;
    } else {
        ctx.send(
            CreateReply::default()
                .content("I've sent the message.")
                .ephemeral(true),
        )
        .await?;
    }

    return Ok(());
}

/// Get the icon of a custom emoji.
#[poise::command(
    slash_command,
    required_bot_permissions = "READ_MESSAGE_HISTORY | VIEW_CHANNEL"
)]
pub async fn edit(
    ctx: Context<'_>,
    #[description = "The message you want to edit."] mut message_id: Message,
    #[max_length = 2000]
    #[description = "Contents of the message."]
    content: Option<String>,
    #[description = "Embeds of the message."] embeds: Option<String>,
) -> Result<(), Error> {
    if message_id.author != **ctx.cache().current_user() {
        ctx.send(
            CreateReply::default()
                .content("Please provide a message sent by me.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    if message_id.kind != MessageType::Regular {
        ctx.send(
            CreateReply::default()
                .content("My message must not be from a command.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let mut single_embed = None;
    let mut multiple_embeds = None;

    if let Some(embeds) = embeds {
        single_embed = serde_json::from_str::<Embed>(&embeds).ok();
        if single_embed.is_none() {
            multiple_embeds = serde_json::from_str::<Vec<Embed>>(&embeds).ok();
            if multiple_embeds.is_none() {
                ctx.send(
                    CreateReply::default()
                        .content("Please send valid embed data.")
                        .ephemeral(true),
                )
                .await?;
            }
        }
    }

    let mut edit_message = EditMessage::new();

    if let Some(embed) = single_embed {
        edit_message = edit_message.embed(CreateEmbed::from(embed));
    } else if let Some(embeds) = multiple_embeds {
        for embed in embeds {
            edit_message = edit_message.embed(CreateEmbed::from(embed));
        }
    }
    if let Some(content) = content {
        edit_message = edit_message.content(content);
    } else {
        edit_message = edit_message.content("");
    }

    let message_result = message_id.edit(ctx, edit_message).await;

    if let Err(err) = message_result {
        ctx.send(
            CreateReply::default()
                .content(format!("An error has occured:\n{}", err))
                .ephemeral(true),
        )
        .await?;
    } else {
        ctx.send(
            CreateReply::default()
                .content("I've edited the message.")
                .ephemeral(true),
        )
        .await?;
    }

    return Ok(());
}
