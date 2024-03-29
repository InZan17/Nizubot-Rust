use poise::serenity_prelude::{AttachmentType, CreateEmbed, Embed, Message, MessageType};

use crate::{Context, Error};

/// Commands for messages.
#[poise::command(
    slash_command,
    subcommands("edit", "clean", "analyze"),
    subcommand_required,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn message(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Get information about a message.
#[poise::command(slash_command)]
pub async fn analyze(
    ctx: Context<'_>,
    #[description = "The message you want to get information from."] message_id: Message,
) -> Result<(), Error> {
    let data = serde_json::to_string_pretty(&message_id).unwrap();
    let data_bytes = data.as_bytes();
    ctx.send(|m| {
        m.attachment(AttachmentType::Bytes {
            data: std::borrow::Cow::Borrowed(data_bytes),
            filename: "message.txt".to_string(),
        })
    })
    .await?;

    Ok(())
}

/// I will say what you want but not show that you ran a command.
#[poise::command(slash_command)]
pub async fn clean(
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
                ctx.send(|m| m.content("Please send valid embed data.").ephemeral(true))
                    .await?;
            }
        }
    }

    let message_result = ctx
        .channel_id()
        .send_message(ctx, |m| {
            if let Some(embed) = single_embed {
                m.embed(|e| {
                    *e = CreateEmbed::from(embed);
                    e
                });
            } else if let Some(embeds) = multiple_embeds {
                for embed in embeds {
                    m.embed(|e| {
                        *e = CreateEmbed::from(embed);
                        e
                    });
                }
            }
            if let Some(content) = content {
                m.content(content);
            } else if empty_workaround {
                m.content("**\n**");
            }
            m
        })
        .await;

    if let Err(err) = message_result {
        ctx.send(|m| {
            m.content(format!("An error has occured:\n{}", err))
                .ephemeral(true)
        })
        .await?;
    } else {
        ctx.send(|m| m.content("I've sent the message.").ephemeral(true))
            .await?;
    }

    return Ok(());
}

/// Get the icon of a custom emoji.
#[poise::command(slash_command)]
pub async fn edit(
    ctx: Context<'_>,
    #[description = "The message you want to edit."] mut message_id: Message,
    #[max_length = 2000]
    #[description = "Contents of the message."]
    content: Option<String>,
    #[description = "Embeds of the message."] embeds: Option<String>,
) -> Result<(), Error> {
    if !message_id.is_own(ctx) {
        ctx.send(|m| m.content("Please provide a message sent by me."))
            .await?;
        return Ok(());
    }

    if message_id.kind != MessageType::Regular {
        ctx.send(|m| m.content("My message must not be from a command."))
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
                ctx.send(|m| m.content("Please send valid embed data.").ephemeral(true))
                    .await?;
            }
        }
    }

    let message_result = message_id
        .edit(ctx, |m| {
            if let Some(embed) = single_embed {
                m.embed(|e| {
                    *e = CreateEmbed::from(embed);
                    e
                });
            } else if let Some(embeds) = multiple_embeds {
                for embed in embeds {
                    m.embed(|e| {
                        *e = CreateEmbed::from(embed);
                        e
                    });
                }
            }
            if let Some(content) = content {
                m.content(content);
            } else {
                m.content("");
            }
            m
        })
        .await;

    if let Err(err) = message_result {
        ctx.send(|m| {
            m.content(format!("An error has occured:\n{}", err))
                .ephemeral(true)
        })
        .await?;
    } else {
        ctx.send(|m| m.content("I've edited the message.").ephemeral(true))
            .await?;
    }

    return Ok(());
}
