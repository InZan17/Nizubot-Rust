use std::collections::HashMap;

use poise::serenity_prelude::{
    AttachmentType, CreateEmbed, Embed, Emoji, Message, MessageType, Role, ReactionType,
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
#[poise::command(slash_command)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "ID of the message."] message_id: Message,
    #[description = "The emoji to react with."] emoji: ReactionType,
    #[description = "Role to give."] role: Role,
) -> Result<(), Error> {
    println!("{emoji}");
    //TODO: use the react error return as a way to validate if the emoji is actually valid or not.
    message_id.react(ctx, emoji.clone()).await?;

    let guild_id = ctx.guild_id().unwrap();
    let message_id_string = message_id.id.to_string();

    let message_reaction_roles = ctx
        .data()
        .storage_manager
        .get_data_or_default::<HashMap<String, u64>>(
            vec![
                "guilds",
                &guild_id.to_string(),
                "messages",
                &message_id_string,
                "reaction_roles",
            ],
            HashMap::new(),
        )
        .await;

    let mut message_reaction_roles_mut = message_reaction_roles.get_data_mut().await;

    if let Some(role_id) = message_reaction_roles_mut.get(&emoji.as_data()) {
        // TODO: Check if role still exists. Also double check that the bot is not reacted to the reaction. If it isnt then the reaction role should've been removed.
        ctx.send(|m| {
            m.content(format!(
                "This emoji already has a role assigned to it. <@&{}>",
                role_id
            ))
            .ephemeral(true)
        })
        .await?;
        return Ok(());
    }

    message_reaction_roles_mut.insert(emoji.as_data(), *role.id.as_u64());
    message_reaction_roles.request_file_write().await;

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
    //TODO: See the TODO in the other function.
    message_id
        .delete_reaction(ctx, Some(ctx.framework().bot_id), emoji.clone())
        .await?;

    let guild_id = ctx.guild_id().unwrap();
    let message_id_string = message_id.id.to_string();

    let message_reaction_roles = ctx
        .data()
        .storage_manager
        .get_data_or_default::<HashMap<String, u64>>(
            vec![
                "guilds",
                &guild_id.to_string(),
                "messages",
                &message_id_string,
                "reaction_roles",
            ],
            HashMap::new(),
        )
        .await;

    let mut message_reaction_roles_mut = message_reaction_roles.get_data_mut().await;

    let Some(role_id) = message_reaction_roles_mut.remove(&emoji.as_data()) else {
        ctx.send(|m| {
            m.content("This message doesn't have this reaction.")
                .ephemeral(true)
        })
        .await?;
        return Ok(());
    };

    message_reaction_roles.request_file_write().await;

    ctx.send(|m| {
        m.content(format!(
            "Sucessfully removed reaction role! <@&{}>",
            role_id
        ))
        .ephemeral(true)
    })
    .await?;

    Ok(())
}
