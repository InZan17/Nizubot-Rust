use std::{collections::HashMap, sync::Arc};

use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};
use poise::serenity_prelude::{
    self, Context, EmojiId, GuildId, Member, MessageId, Reaction, ReactionType, RoleId, UserId,
};
use serde::Deserialize;

use crate::Error;

use super::db::SurrealClient;

pub struct ReactionManager {
    pub db: Arc<SurrealClient>,
}

#[derive(Deserialize)]
pub struct ReactionRoles {
    pub reaction_roles: Option<HashMap<String, RoleId>>,
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
        message_id: MessageId,
    ) -> Result<(), ReactionError> {
        let db = &self.db;

        let message_reaction_roles_option =
            match db.get_message_reaction_roles(&guild_id, &message_id).await {
                Ok(ok) => ok,
                Err(err) => {
                    return Err(ReactionError::Database(
                        err,
                        "Couldn't fetch current reaction roles from database.".to_string(),
                    ))
                }
            };

        if let Some(message_reaction_roles) = message_reaction_roles_option {
            if let Some(role_id) = message_reaction_roles.get(&get_emoji_id(&emoji)) {
                return Err(ReactionError::EmojiTaken(*role_id));
            }

            // Check if role is already registered on the same message.
            for (other_emoji, other_role_id) in message_reaction_roles.iter() {
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
        };

        let emoji_id = get_emoji_id(&emoji);

        if let Err(err) = db
            .set_message_reaction_role(&guild_id, &message_id, &emoji_id, Some(&role_id))
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
        //TODO/SUGGESTION: Allow for removing using roles too. Make an enum which has either reactiontype or roleid
        emoji: ReactionType,
        guild_id: GuildId,
        message_id: MessageId,
    ) -> Result<RoleId, ReactionError> {
        let db = &self.db;

        let emoji_id = get_emoji_id(&emoji);

        let role_id = match db
            .get_role_from_message_reaction(&guild_id, &message_id, &emoji_id)
            .await
        {
            Ok(ok) => ok,
            Err(err) => {
                return Err(ReactionError::Database(
                    err,
                    "Couldn't get role id from emoji from database.".to_string(),
                ))
            }
        };

        let Some(role_id) = role_id else {
            return Err(ReactionError::NoReaction);
        };

        if let Err(err) = db
            .set_message_reaction_role(&guild_id, &message_id, &emoji_id, None)
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

        let message_reaction_roles =
            match db.get_message_reaction_roles(&guild_id, &message_id).await {
                Ok(ok) => ok,
                Err(err) => {
                    return Err(ReactionError::Database(
                        err,
                        "Couldn't fetch current reaction roles from database.".to_string(),
                    ))
                }
            }
            .unwrap_or_default();

        Ok(message_reaction_roles)
    }

    /// Gets all reaction role messages in a guild and the amount of reaction roles on them.
    pub async fn get_reaction_role_messages(
        &self,
        guild_id: GuildId,
    ) -> Result<Vec<(MessageId, usize)>, ReactionError> {
        let db = &self.db;

        let messages = match db.get_reaction_role_messages(&guild_id).await {
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

        for (message_id, reaction_roles) in messages.into_iter() {
            let Some(reaction_roles) = &reaction_roles.reaction_roles else {
                continue;
            };

            if reaction_roles.len() != 0 {
                filtered_messages.push((message_id, reaction_roles.len()));
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
        let role_id = match db
            .get_role_from_message_reaction(&guild_id, &message_id, &emoji_id)
            .await
        {
            Ok(ok) => ok,
            Err(err) => {
                return Err(ReactionError::Database(
                    err,
                    "Couldn't get role id from emoji from database.".to_string(),
                ))
            }
        };

        let Some(role_id) = role_id else {
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

        let role_id = match db
            .get_role_from_message_reaction(&guild_id, &message_id, &emoji_id)
            .await
        {
            Ok(ok) => ok,
            Err(err) => {
                return Err(ReactionError::Database(
                    err,
                    "Couldn't get role id from emoji from database.".to_string(),
                ))
            }
        };

        let Some(role_id) = role_id else {
            return Ok(());
        };

        if user_id == bot_id {
            if let Err(err) = db
                .set_message_reaction_role(&guild_id, &message_id, &emoji_id, None)
                .await
            {
                return Err(ReactionError::Database(
                    err,
                    "Bot unreacted to reaction. Couldn't remove reaction role from message."
                        .to_string(),
                ));
            };
            return Err(ReactionError::BotReactionRemoved(role_id, emoji_id));
        }

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
