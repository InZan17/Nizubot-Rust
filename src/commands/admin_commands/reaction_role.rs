use std::collections::HashMap;

use poise::serenity_prelude::{
    AttachmentType, CreateEmbed, Embed, Emoji, Message, MessageId, MessageType, ReactionType, Role,
};

use crate::{Context, Error};

/// Manage reactions so you get roles when clicking them.
#[poise::command(
    slash_command,
    subcommands("add", "remove", "list"),
    subcommand_required,
    guild_only,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn reaction_role(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Add reaction role to message.
#[poise::command(
    slash_command,
    required_bot_permissions = "VIEW_CHANNEL | READ_MESSAGE_HISTORY | MANAGE_ROLES | ADD_REACTIONS"
)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "ID of the message."] message_id: Message,
    #[description = "The emoji to react with."] emoji: ReactionType,
    #[description = "Role to give."] role: Role,
) -> Result<(), Error> {
    if let Err(err) = message_id.react(ctx, emoji.clone()).await {
        ctx.send(|m| {
            m.content(format!("Sorry, I couldn't react with the emoji you provided. Please make sure to provide an actual emoji.\n\nHere's the error: {}", err)).ephemeral(true)
        }).await?;
        return Ok(());
    }

    let guild_id = ctx.guild_id().unwrap();
    let message_id = message_id.id;

    let res = ctx
        .data()
        .reaction_manager
        .add_reaction(emoji, role.id, guild_id, message_id)
        .await;

    if let Err(err) = res {
        ctx.send(|m| {
            m.content(format!(
                "Sorry, I wasn't able to add that reaction role.\n\n{}",
                err.to_string()
            ))
            .ephemeral(true)
        })
        .await?;
        return Ok(());
    }

    ctx.send(|m| {
        m.content(format!("Sucessfully added reaction role!\nTo remove the reaction role, simply remove my reaction or run `/reaction_role remove`.")).ephemeral(true)
    }).await?;

    Ok(())
}

/// Remove reaction role from message.
#[poise::command(
    slash_command,
    required_bot_permissions = "VIEW_CHANNEL | READ_MESSAGE_HISTORY"
)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "ID of the message."] message_id: Message,
    #[description = "The emoji to remove."] emoji: ReactionType,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let message_id_id = message_id.id;

    let removed_role_res = ctx
        .data()
        .reaction_manager
        .remove_reaction(emoji.clone(), guild_id, message_id_id)
        .await;

    match removed_role_res {
        Ok(removed_role) => {
            let _ = message_id
                .delete_reaction(ctx, Some(ctx.framework().bot_id), emoji.clone())
                .await;

            ctx.send(|m| {
                m.content(format!(
                    "Sucessfully removed reaction role! <@&{}>",
                    removed_role
                ))
                .ephemeral(true)
            })
            .await?;
        }
        Err(err) => {
            ctx.send(|m| {
                m.content(format!(
                    "Sorry, I wasn't able to remove that reaction role.\n\n{}",
                    err.to_string()
                ))
                .ephemeral(true)
            })
            .await?;
        }
    }

    Ok(())
}

/// List all reaction roles in this guild or for a message.
#[poise::command(slash_command)]
pub async fn list(ctx: Context<'_>, message_id: Option<MessageId>) -> Result<(), Error> {
    if let Some(message_id) = message_id {
        let reaction_roles = ctx
            .data()
            .reaction_manager
            .get_reaction_roles(ctx.guild_id().unwrap(), message_id)
            .await;

        match reaction_roles {
            Err(err) => {
                ctx.send(|m| {
                    m.content(format!(
                        "Sorry, I wasn't able to list the message's reaction roles.\n\n{}",
                        err.to_string()
                    ))
                    .ephemeral(true)
                })
                .await?;
                return Ok(());
            }
            Ok(reaction_roles) => {
                let mut keys = reaction_roles.keys().collect::<Vec<_>>();
                keys.sort();
                ctx.send(|m| {
                    m.embed(|e| {
                        e.title("Reaction Roles")
                            .description("All of the reaction roles for this message.")
                            .footer(|f| f.text(format!("Total reactors: {}", keys.len())));

                        let mut description = String::new();
                        for key in keys {
                            let role_id = reaction_roles.get(key).unwrap();

                            let is_custom_emoji = key.chars().all(char::is_numeric);

                            let emoji;
                            let raw_emoji;

                            if is_custom_emoji {
                                raw_emoji = key.clone();
                                emoji = format!("<:custom:{key}>");
                            } else {
                                raw_emoji = format!("\\{key}");
                                emoji = key.clone();
                            }

                            description =
                                format!("{description}{emoji} ({raw_emoji}): <@&{role_id}>\n");
                        }

                        e.description(description);

                        e
                    })
                    .ephemeral(true)
                })
                .await?;
            }
        }
    } else {
        let reaction_messages = ctx
            .data()
            .reaction_manager
            .get_reaction_role_messages(ctx.guild_id().unwrap())
            .await;

        match reaction_messages {
            Err(err) => {
                ctx.send(|m| {
                    m.content(format!(
                        "Sorry, I wasn't able to list reaction role messages in this guild.\n\n{}",
                        err.to_string()
                    ))
                    .ephemeral(true)
                })
                .await?;
                return Ok(());
            }
            Ok(reaction_messages) => {
                ctx.send(|m| {
                    m.embed(|e| {
                        e.title("Reaction Roles Messages")
                            .description("All of the reaction role messages for this guild.")
                            .footer(|f| {
                                f.text(format!("Total messages: {}", reaction_messages.len()))
                            });

                        let mut description = String::new();
                        for (message_id, reaction_count) in reaction_messages {
                            description = format!(
                                "{description}{message_id}: {reaction_count} reaction role{}.\n",
                                if reaction_count == 1 { "" } else { "s" }
                            );
                        }

                        e.description(description);

                        e
                    })
                    .ephemeral(true)
                })
                .await?;
            }
        }
    }

    return Ok(());
}
