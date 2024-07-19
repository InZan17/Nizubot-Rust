use std::{collections::HashMap, sync::Arc};

use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};
use poise::serenity_prelude::{
    self, ChannelId, Context, EmojiId, GuildId, Member, MessageId, Reaction, ReactionType, RoleId,
    UserId,
};
use serde::Deserialize;

use crate::Error;

use super::{db::SurrealClient, message_manager::StoredMessageData};

pub struct ReactionManager {
    pub db: Arc<SurrealClient>,
}

pub enum ReactionError {
    NoReaction,
    NoRole,
    EmojiTaken(RoleId),
    RoleTaken(String),
    Database(Error, String),
    Serenity(serenity_prelude::Error, String),
    BotReactionRemoved(RoleId, String),
}

impl ReactionError {
    pub fn to_string(&self) -> String {
        match self {
            ReactionError::NoReaction => "This message doesn't have this reaction.".to_string(),
            ReactionError::NoRole => "This message doesn't have this role.".to_string(),
            ReactionError::EmojiTaken(role_id) => {
                format!("This emoji already has a role assigned to it: <@&{role_id}>")
            }
            ReactionError::RoleTaken(emoji) => {
                format!("This role already has an emoji assigned to it: {emoji}")
            }
            ReactionError::Database(err, description) => format!("{description} {err}"),
            ReactionError::BotReactionRemoved(role_id, emoji_str) => format!("Bot reaction has been removed. This will unregister the role {role_id} to the reaction {emoji_str}."),
            ReactionError::Serenity(err, description) => format!("{description} {err}"),
        }
    }
}

pub enum ReactionTypeOrRoleId {
    ReactionType(ReactionType),
    RoleId(RoleId),
}

impl From<ReactionType> for ReactionTypeOrRoleId {
    fn from(value: ReactionType) -> Self {
        Self::ReactionType(value)
    }
}

impl From<RoleId> for ReactionTypeOrRoleId {
    fn from(value: RoleId) -> Self {
        Self::RoleId(value)
    }
}

impl ReactionManager {
    pub fn new(db: Arc<SurrealClient>) -> Self {
        Self { db }
    }

    /// Adds a reaction event to a message and makes it so anyone that react to the reaction gets a role.
    ///
    /// Errors if communication with db doesn't work or if a role/emoji is already registered for that message.
    pub async fn add_reaction(
        &self,
        emoji: ReactionType,
        role_id: RoleId,
        guild_id: GuildId,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<(), ReactionError> {
        let db = &self.db;

        let guild_message_option = match db.get_guild_message(&guild_id, &message_id).await {
            Ok(ok) => ok,
            Err(err) => {
                return Err(ReactionError::Database(
                    err,
                    "Couldn't fetch current reaction roles from database.".to_string(),
                ))
            }
        };

        let mut guild_message;

        if let Some(some_guild_message) = guild_message_option {
            guild_message = some_guild_message;
        } else {
            guild_message = StoredMessageData {
                message_id: Some(message_id),
                channel_id: Some(channel_id),
                reaction_roles: HashMap::new(),
            }
        }

        if guild_message.needs_updating() {
            guild_message.message_id = Some(message_id);
            guild_message.channel_id = Some(channel_id);
        }

        if let Some(role_id) = guild_message.reaction_roles.get(&get_emoji_id(&emoji)) {
            return Err(ReactionError::EmojiTaken(*role_id));
        }

        // Check if role is already registered on the same message.
        for (other_emoji, other_role_id) in guild_message.reaction_roles.iter() {
            if *other_role_id != role_id {
                continue;
            }

            let emoji_string = if other_emoji.chars().all(char::is_numeric) {
                format!("<:custom:{other_emoji}>")
            } else {
                other_emoji.clone()
            };

            return Err(ReactionError::RoleTaken(emoji_string));
        }

        let emoji_id = get_emoji_id(&emoji);

        guild_message.reaction_roles.insert(emoji_id, role_id);

        if let Err(err) = db
            .set_guild_message(&guild_id, &message_id, Some(&guild_message))
            .await
        {
            return Err(ReactionError::Database(
                err,
                "Couldn't update reaction role.".to_string(),
            ));
        };
        Ok(())
    }

    /// Removes a reaction event to a message.
    ///
    /// Errors if communication with db doesn't work or if there's no emoji registered for that message.
    pub async fn remove_reaction(
        &self,
        emoji_or_role: ReactionTypeOrRoleId,
        guild_id: GuildId,
        message_id: MessageId,
    ) -> Result<(RoleId), ReactionError> {
        let db = &self.db;

        let guild_message = match db.get_guild_message(&guild_id, &message_id).await {
            Ok(ok) => ok,
            Err(err) => {
                return Err(ReactionError::Database(
                    err,
                    "Couldn't get role id from emoji from database.".to_string(),
                ))
            }
        };

        let Some(mut guild_message) = guild_message else {
            return Err(ReactionError::NoReaction);
        };

        let mut emoji_id_removal = String::new();

        match emoji_or_role {
            ReactionTypeOrRoleId::ReactionType(emoji) => {
                emoji_id_removal = get_emoji_id(&emoji);
            }
            ReactionTypeOrRoleId::RoleId(role_id) => {
                for (key, v) in guild_message.reaction_roles.iter() {
                    if *v == role_id {
                        emoji_id_removal = key.clone();
                    }
                }
                if emoji_id_removal.is_empty() {
                    return Err(ReactionError::NoReaction);
                }
            }
        }

        let role_id = guild_message.reaction_roles.remove(&emoji_id_removal);

        let Some(role_id) = role_id else {
            return Err(ReactionError::NoReaction);
        };

        if let Err(err) = db
            .set_guild_message(&guild_id, &message_id, Some(&guild_message))
            .await
        {
            return Err(ReactionError::Database(
                err,
                "Couldn't remove reaction role from message.".to_string(),
            ));
        }

        Ok(role_id)
    }

    /// Gets all reaction roles given a guild id and message id.
    pub async fn get_reaction_roles(
        &self,
        guild_id: GuildId,
        message_id: MessageId,
    ) -> Result<HashMap<String, RoleId>, ReactionError> {
        let db = &self.db;

        let guild_message = match db.get_guild_message(&guild_id, &message_id).await {
            Ok(ok) => ok,
            Err(err) => {
                return Err(ReactionError::Database(
                    err,
                    "Couldn't fetch current reaction roles from database.".to_string(),
                ))
            }
        };

        let reaction_roles = match guild_message {
            Some(some) => some.reaction_roles,
            None => HashMap::new(),
        };

        Ok(reaction_roles)
    }

    /// Gets all reaction role messages in a guild and the amount of reaction roles on them.
    pub async fn get_reaction_role_messages(
        &self,
        guild_id: GuildId,
    ) -> Result<Vec<(MessageId, Option<ChannelId>, usize)>, ReactionError> {
        let db = &self.db;

        let messages = match db.get_guild_messages(&guild_id).await {
            Ok(ok) => ok,
            Err(err) => {
                return Err(ReactionError::Database(
                    err,
                    "Couldn't fetch current reaction roles from database.".to_string(),
                ))
            }
        }
        .unwrap_or_default();

        let mut filtered_messages = Vec::with_capacity(messages.len());

        for (message_id, guild_message) in messages.into_iter() {
            if guild_message.reaction_roles.len() != 0 {
                filtered_messages.push((
                    message_id,
                    guild_message.channel_id,
                    guild_message.reaction_roles.len(),
                ));
            }
        }

        filtered_messages.sort();

        Ok(filtered_messages)
    }

    /// Runs whenever a user reacts to a message.
    /// Will check for if the reaction has a registered role to it and then add that role to user.
    ///
    /// Errors if communication to db doesn't work or if adding role to user doesn't work.
    pub async fn reaction_add_event(
        &self,
        ctx: &Context,
        reaction: &Reaction,
        bot_id: UserId,
    ) -> Result<(), ReactionError> {
        let Some(guild_id) = reaction.guild_id else {
            return Ok(());
        };

        let Some(user_id) = reaction.user_id else {
            //This should never happen.
            return Ok(());
        };

        if user_id == bot_id {
            return Ok(());
        }

        let db = &self.db;

        let message_id = reaction.message_id;

        let emoji_id = get_emoji_id(&reaction.emoji);

        // TODO: try caching the results to not do as many calls to the db.
        let guild_message_option = match db.get_guild_message(&guild_id, &message_id).await {
            Ok(ok) => ok,
            Err(err) => {
                return Err(ReactionError::Database(
                    err,
                    "Couldn't get role id from emoji from database.".to_string(),
                ))
            }
        };

        let Some(mut guild_message) = guild_message_option else {
            return Ok(());
        };

        if guild_message.needs_updating() {
            guild_message.message_id = Some(reaction.message_id);
            guild_message.channel_id = Some(reaction.channel_id);
            let _ = db
                .set_guild_message(&guild_id, &message_id, Some(&guild_message))
                .await;
        }

        let Some(role_id) = guild_message.reaction_roles.get(&emoji_id) else {
            return Ok(());
        };

        let mut member = match guild_id.member(&ctx, user_id).await {
            Ok(ok) => ok,
            Err(err) => {
                return Err(ReactionError::Serenity(
                    err,
                    "Couldn't fetch member from guild.".to_string(),
                ))
            }
        };

        if let Err(err) = member.add_role(&ctx, role_id).await {
            return Err(ReactionError::Serenity(
                err,
                "Couldn't assign role to member.".to_string(),
            ));
        };

        Ok(())
    }

    /// Runs whenever a user unreacts to a message.
    /// Will check for if the reaction has a registered role to it and then remove that role from user.
    ///
    /// Errors if communication to db doesn't work or if removing role from user doesn't work.
    pub async fn reaction_remove_event(
        &self,
        ctx: &Context,
        reaction: &Reaction,
        bot_id: UserId,
    ) -> Result<(), ReactionError> {
        let Some(guild_id) = reaction.guild_id else {
            return Ok(());
        };

        let Some(user_id) = reaction.user_id else {
            //This should never happen.
            return Ok(());
        };

        let message_id = reaction.message_id;

        let emoji_id = get_emoji_id(&reaction.emoji);

        let db = &self.db;

        let guild_message_option = match db.get_guild_message(&guild_id, &message_id).await {
            Ok(ok) => ok,
            Err(err) => {
                return Err(ReactionError::Database(
                    err,
                    "Couldn't get role id from emoji from database.".to_string(),
                ))
            }
        };

        let Some(mut guild_message) = guild_message_option else {
            return Ok(());
        };

        let needs_updating = guild_message.needs_updating();

        if needs_updating {
            guild_message.channel_id = Some(reaction.channel_id);
            guild_message.message_id = Some(reaction.message_id);
        }

        if user_id == bot_id {
            let Some(removed_role_id) = guild_message.reaction_roles.remove(&emoji_id) else {
                return Ok(());
            };

            if let Err(err) = db
                .set_guild_message(&guild_id, &message_id, Some(&guild_message))
                .await
            {
                return Err(ReactionError::Database(
                    err,
                    "Bot unreacted to reaction but couldn't remove reaction role from message."
                        .to_string(),
                ));
            };
            return Err(ReactionError::BotReactionRemoved(removed_role_id, emoji_id));
        } else if needs_updating {
            let _ = db
                .set_guild_message(&guild_id, &message_id, Some(&guild_message))
                .await;
        }

        let Some(role_id) = guild_message.reaction_roles.get(&emoji_id).cloned() else {
            return Ok(());
        };

        let mut member = match guild_id.member(&ctx, user_id).await {
            Ok(ok) => ok,
            Err(err) => {
                return Err(ReactionError::Serenity(
                    err,
                    "Couldn't fetch member from guild.".to_string(),
                ))
            }
        };

        if let Err(err) = member.remove_role(&ctx, role_id).await {
            return Err(ReactionError::Serenity(
                err,
                "Couldn't remove role from member.".to_string(),
            ));
        };

        Ok(())
    }
}

/// Returns the id of a custom emoji, not including its name.
/// If it's an unicode emoji it will return the unicode emoji.
fn get_emoji_id(emoji: &ReactionType) -> String {
    match emoji {
        ReactionType::Custom {
            animated: _,
            id,
            name: _,
        } => id.0.to_string(),
        ReactionType::Unicode(name) => name.to_string(),
        _ => emoji.as_data(),
    }
}
