use std::{sync::Arc, collections::HashMap};

use poise::serenity_prelude::{Context, Reaction, RoleId, Member};

use super::storage_manager::StorageManager;

pub struct ReactionManager {
    pub storage_manager: Arc<StorageManager>,
}

impl ReactionManager {
    pub fn new(storage_manager: Arc<StorageManager>) -> Self {
        Self { storage_manager }
    }

    pub async fn reaction_add(&self, ctx: &Context, reaction: &Reaction) {
        let Some(guild_id) = reaction.guild_id else {
            return;
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

        let message_reaction_roles_read = message_reaction_roles.get_data().await;

        let Some(role_id) = message_reaction_roles_read.get(&reaction.emoji.as_data()) else {
            return
        };

        let Some(user_id) = reaction.user_id else {
            return
        };

        let Ok(mut member) = guild_id.member(&ctx, user_id).await else {
            return
        };

        // TODO: If this fails, send a notification to the servers error log.
        let res = member.add_role(&ctx, RoleId(*role_id)).await;     
    }

    pub async fn reaction_remove(&self, ctx: &Context, reaction: &Reaction) {
        //TODO: Check if the removed reaction is self.
        let Some(guild_id) = reaction.guild_id else {
            return;
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

        let message_reaction_roles_read = message_reaction_roles.get_data().await;

        let Some(role_id) = message_reaction_roles_read.get(&reaction.emoji.as_data()) else {
            return
        };

        let Some(user_id) = reaction.user_id else {
            return
        };

        let Ok(mut member) = guild_id.member(&ctx, user_id).await else {
            return
        };

        // TODO: If this fails, send a notification to the servers error log.
        let res = member.remove_role(&ctx, RoleId(*role_id)).await;     
    }
}
