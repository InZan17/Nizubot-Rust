use std::{collections::HashMap, sync::Arc};

use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};
use poise::serenity_prelude::{Context, Member, Reaction, ReactionType, RoleId, UserId};

use crate::Error;

use super::storage_manager::StorageManager;

pub struct ReactionManager {
    pub storage_manager: Arc<StorageManager>,
}

impl ReactionManager {
    pub fn new(storage_manager: Arc<StorageManager>) -> Self {
        Self { storage_manager }
    }

    pub async fn add_reaction(
        &self,
        emoji: ReactionType,
        role_id: u64,
        guild_id: u64,
        message_id: u64,
    ) -> Result<(), Error> {
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

        let mut message_reaction_roles_mut = message_reaction_roles.get_data_mut().await;

        if let Some(role_id) = message_reaction_roles_mut.get(&get_emoji_id(&emoji)) {
            return Err(
                format!("This emoji already has a role assigned to it. <@&{role_id}>").into(),
            );
        }

        for (other_emoji, other_role_id) in message_reaction_roles_mut.iter() {
            if *other_role_id != role_id {
                continue;
            }

            let emoji_string = if other_emoji.chars().all(char::is_numeric) {
                format!("<:_:{other_emoji}>")
            } else {
                percent_decode_str(other_emoji)
                    .decode_utf8_lossy()
                    .to_string()
            };
            return Err(
                format!("This role already has an emoji assigned to it. {emoji_string}").into(),
            );
        }

        message_reaction_roles_mut.insert(get_emoji_id(&emoji), role_id);
        message_reaction_roles.request_file_write().await;
        Ok(())
    }

    pub async fn remove_reaction(
        &self,
        emoji: ReactionType,
        guild_id: u64,
        message_id: u64,
    ) -> Result<(u64), Error> {
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

        let mut message_reaction_roles_mut = message_reaction_roles.get_data_mut().await;

        let Some(role_id) = message_reaction_roles_mut.remove(&emoji.as_data()) else {
            return Err("This message doesn't have this reaction.".into());
        };

        message_reaction_roles.request_file_write().await;
        Ok(role_id)
    }

    pub async fn reaction_add_event(
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

        let Some(role_id) = message_reaction_roles_read.get(&get_emoji_id(&reaction.emoji)) else {
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

    pub async fn reaction_remove_event(
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
            message_reaction_roles_mut.remove(&get_emoji_id(&reaction.emoji));
            message_reaction_roles.request_file_write().await;
            return Err("Bot reaction has been removed. This also removes the ability to get roles from the reaction.".into());
        }

        let message_reaction_roles_read = message_reaction_roles.get_data().await;

        let Some(role_id) = message_reaction_roles_read.get(&get_emoji_id(&reaction.emoji)) else {
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

fn get_emoji_id(emoji: &ReactionType) -> String {
    match emoji {
        ReactionType::Custom {
            animated: _,
            id,
            name: _,
        } => id.0.to_string(),
        ReactionType::Unicode(name) => utf8_percent_encode(name, NON_ALPHANUMERIC).to_string(),
        _ => emoji.as_data(),
    }
}
