use std::collections::HashMap;

use poise::serenity_prelude::{
    AttachmentType, CreateEmbed, Embed, Emoji, Message, MessageType, ReactionType, Role,
};

use crate::{Context, Error};

/// Manage reactions so you get roles when clicking them.
#[poise::command(
    slash_command,
    subcommands("add", "remove"),
    subcommand_required,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn reaction_role(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Add reaction role to message.
#[poise::command(slash_command, guild_only)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "ID of the message."] message_id: Message,
    #[description = "The emoji to react with."] emoji: ReactionType,
    #[description = "Role to give."] role: Role,
) -> Result<(), Error> {
    if let Err(err) = message_id.react(ctx, emoji.clone()).await {
        ctx.send(|m| {
            m.content(format!("Sorry, I couldn't react with the emoji you provided. Please make sure to provide an actual emoji.\n\nHere's the error: {}", err))
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
            m.ephemeral(true).content(format!(
                "Sorry, I wasn't able to add that reaction role.\n\n{}",
                err.to_string()
            ))
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
#[poise::command(slash_command)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "ID of the message."] message_id: Message,
    #[description = "The emoji to remove."] emoji: ReactionType,
) -> Result<(), Error> {
    //Unreacting is not as important as reacting. Therefor we do not need to error out if reaction deletion doesnt work.
    let _ = message_id
        .delete_reaction(ctx, Some(ctx.framework().bot_id), emoji.clone())
        .await;

    let guild_id = ctx.guild_id().unwrap();
    let message_id = message_id.id;

    let removed_role_res = ctx
        .data()
        .reaction_manager
        .remove_reaction(emoji, guild_id, message_id)
        .await;

    match removed_role_res {
        Ok(removed_role) => {
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
                m.ephemeral(true).content(format!(
                    "Sorry, I wasn't able to remove that reaction role.\n\n{}",
                    err.to_string()
                ))
            })
            .await?;
        }
    }

    Ok(())
}
