use std::{collections::HashMap, sync::Arc};

use poise::serenity_prelude::{Context, Member, Reaction, RoleId, UserId};

use crate::Error;

use super::storage_manager::StorageManager;

pub struct ReactionManager {
    pub storage_manager: Arc<StorageManager>,
}

impl ReactionManager {
    pub fn new(storage_manager: Arc<StorageManager>) -> Self {
        Self { storage_manager }
    }

    pub async fn reaction_add(
        &self,
        ctx: &Context,
        reaction: &Reaction,
        bot_id: UserId,
    ) -> Result<(), Error> {
        let Some(guild_id) = reaction.guild_id else {
            return Ok(());
        };

        let Some(user_id) = reaction.user_id else {
            return Err("Couldn't get the UserId from a reaction.".into());
        };

        if user_id == bot_id {
            return Ok(());
        }

        let message_id = reaction.message_id;

        let message_reaction_roles = self
            .storage_manager
            .get_data_or_default::<HashMap<String, u64>>(
                vec![
                    "guilds",
                    &guild_id.to_string(),
                    "messages",
                    &message_id.to_string(),
                    "reaction_roles",
                ],
                HashMap::new(),
            )
            .await;

        let message_reaction_roles_read = message_reaction_roles.get_data().await;

        let Some(role_id) = message_reaction_roles_read.get(&reaction.emoji.as_data()) else {
            return Ok(());
        };

        let mut member = guild_id.member(&ctx, user_id).await?;

        let res = member.add_role(&ctx, RoleId(*role_id)).await;

        if let Err(err) = res {
            return Err(err.into());
        } else {
            return Ok(());
        }
    }

    pub async fn reaction_remove(
        &self,
        ctx: &Context,
        reaction: &Reaction,
        bot_id: UserId,
    ) -> Result<(), Error> {
        let Some(guild_id) = reaction.guild_id else {
            return Ok(());
        };

        let message_id = reaction.message_id;

        let message_reaction_roles = self
            .storage_manager
            .get_data_or_default::<HashMap<String, u64>>(
                vec![
                    "guilds",
                    &guild_id.to_string(),
                    "messages",
                    &message_id.to_string(),
                    "reaction_roles",
                ],
                HashMap::new(),
            )
            .await;

        let Some(user_id) = reaction.user_id else {
            return Err("Couldn't get the UserId from a reaction.".into());
        };

        if user_id == bot_id {
            let mut message_reaction_roles_mut = message_reaction_roles.get_data_mut().await;
            message_reaction_roles_mut.remove(&reaction.emoji.as_data());
            message_reaction_roles.request_file_write().await;
            return Err("Bot reaction has been removed. This also removes the ability to get roles from the reaction.".into());
        }

        let message_reaction_roles_read = message_reaction_roles.get_data().await;

        let Some(role_id) = message_reaction_roles_read.get(&reaction.emoji.as_data()) else {
            return Ok(());
        };

        let mut member = guild_id.member(&ctx, user_id).await?;

        let res = member.remove_role(&ctx, RoleId(*role_id)).await;

        if let Err(err) = res {
            return Err(err.into());
        } else {
            return Ok(());
        }
    }
}
