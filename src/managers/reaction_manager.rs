use std::{collections::HashMap, sync::Arc};

use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};
use poise::serenity_prelude::{Context, Member, Reaction, ReactionType, RoleId, UserId};

use crate::Error;

use super::db::SurrealClient;

pub struct ReactionManager {
    pub db: Arc<SurrealClient>,
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
        role_id: u64,
        guild_id: u64,
        message_id: u64,
    ) -> Result<(), Error> {
        let db = &self.db;

        let table_id = format!("guild:{guild_id}");

        let message_reaction_roles_option: Option<HashMap<String, u64>> = db
            .query(format!(
                "SELECT VALUE messages.{message_id}.reaction_roles from {table_id};"
            ))
            .await?
            .take(0)?;

        if let Some(message_reaction_roles) = message_reaction_roles_option {
            if let Some(role_id) = message_reaction_roles.get(&get_emoji_id(&emoji)) {
                return Err(
                    format!("This emoji already has a role assigned to it. <@&{role_id}>").into(),
                );
            }

            // Check if role is already registered on the same message.
            for (other_emoji, other_role_id) in message_reaction_roles.iter() {
                if *other_role_id != role_id {
                    continue;
                }

                let emoji_string = if other_emoji.chars().all(char::is_numeric) {
                    format!("<:custom:{other_emoji}>")
                } else {
                    // Actual emojis are stored encoded so we decode it.
                    percent_decode_str(other_emoji)
                        .decode_utf8_lossy()
                        .to_string()
                };
                return Err(format!(
                    "This role already has an emoji assigned to it. {emoji_string}"
                )
                .into());
            }
        };

        // TODO: if emoji is unicode the get_emoji_id will return the actual unicode character.
        // We never decode it anywhere. So when getting the emoji_string variable a few lines up, do we really need to decode str?
        let emoji_id = get_emoji_id(&emoji);

        // I have to use merge here because if I try doing ["🧀"] like I do on the other queries then inside the database it will be "'🧀'" instead of "🧀"
        db.query(format!("UPDATE {table_id} MERGE {{ \"messages\": {{ {message_id}: {{ \"reaction_roles\": {{ \"{emoji_id}\": {role_id} }} }} }} }};")).await?;

        Ok(())
    }

    /// Removes a reaction event to a message.
    ///
    /// Errors if communication with db doesn't work or if there's no emoji registered for that message.
    pub async fn remove_reaction(
        &self,
        emoji: ReactionType,
        guild_id: u64,
        message_id: u64,
    ) -> Result<u64, Error> {
        let db = &self.db;

        let table_id = format!("guild:{guild_id}");

        let emoji_id = get_emoji_id(&emoji);

        //TODO: put queries in seperate function.
        let role_id: Option<u64> = db
            .query(format!(
                "SELECT VALUE messages.{message_id}.reaction_roles['{emoji_id}'] from {table_id};"
            ))
            .await?
            .take(0)?;

        let Some(role_id) = role_id else {
            return Err("This message doesn't have this reaction.".into());
        };

        db.query(format!(
            "UPDATE {table_id} SET messages.{message_id}.reaction_roles['{emoji_id}'] = NONE;"
        ))
        .await?;

        Ok(role_id)
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

        let db = &self.db;

        let message_id = reaction.message_id;

        let table_id = format!("guild:{guild_id}");

        let emoji_id = get_emoji_id(&reaction.emoji);

        // TODO: try caching the results to not do as many calls to the db.
        let role_id: Option<u64> = db
            .query(format!(
                "SELECT VALUE messages.{message_id}.reaction_roles['{emoji_id}'] from {table_id};"
            ))
            .await?
            .take(0)?;

        let Some(role_id) = role_id else {
            return Ok(());
        };

        let mut member = guild_id.member(&ctx, user_id).await?;

        member.add_role(&ctx, RoleId(role_id)).await?;

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
    ) -> Result<(), Error> {
        let Some(guild_id) = reaction.guild_id else {
            return Ok(());
        };

        let Some(user_id) = reaction.user_id else {
            return Err("Couldn't get the UserId from a reaction.".into());
        };

        let message_id = reaction.message_id;

        let table_id = format!("guild:{guild_id}");

        let emoji_id = get_emoji_id(&reaction.emoji);

        let db = &self.db;

        let role_id: Option<u64> = db
            .query(format!(
                "SELECT VALUE messages.{message_id}.reaction_roles['{emoji_id}'] from {table_id};"
            ))
            .await?
            .take(0)?;

        let Some(role_id) = role_id else {
            return Ok(());
        };

        if user_id == bot_id {
            db.query(format!(
                "UPDATE {table_id} SET messages.{message_id}.reaction_roles['{emoji_id}'] = NONE;"
            ))
            .await?;
            return Err(format!("Bot reaction has been removed. This will unregister the role {role_id} to the reaction {emoji_id}.").into());
        }

        let mut member = guild_id.member(&ctx, user_id).await?;

        member.remove_role(&ctx, RoleId(role_id)).await?;

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
