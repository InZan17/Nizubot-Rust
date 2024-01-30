use std::{collections::HashMap, sync::Arc};

use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};
use poise::serenity_prelude::{Context, Member, Reaction, ReactionType, RoleId, UserId};
use surrealdb::{engine::remote::ws::Client, Surreal};

use crate::Error;

use super::{db::IsConnected, storage_manager::StorageManager};

pub struct ReactionManager {
    pub db: Arc<Surreal<Client>>,
}

impl ReactionManager {
    pub fn new(db: Arc<Surreal<Client>>) -> Self {
        Self { db }
    }

    pub async fn add_reaction(
        &self,
        emoji: ReactionType,
        role_id: u64,
        guild_id: u64,
        message_id: u64,
    ) -> Result<(), Error> {
        let db = &self.db;

        db.is_connected().await?;

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

            for (other_emoji, other_role_id) in message_reaction_roles.iter() {
                if *other_role_id != role_id {
                    continue;
                }

                let emoji_string = if other_emoji.chars().all(char::is_numeric) {
                    format!("<:custom:{other_emoji}>")
                } else {
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

        let emoji_id = get_emoji_id(&emoji);

        // I have to use merge here because if I try doing ["🧀"] like I do on the other queries then inside the database it will be "'🧀'" instead of "🧀"
        db.query(format!("UPDATE {table_id} MERGE {{ \"messages\": {{ {message_id}: {{ \"reaction_roles\": {{ \"{emoji_id}\": {role_id} }} }} }} }};")).await?;

        Ok(())
    }

    pub async fn remove_reaction(
        &self,
        emoji: ReactionType,
        guild_id: u64,
        message_id: u64,
    ) -> Result<u64, Error> {
        let db = &self.db;

        db.is_connected().await?;

        let table_id = format!("guild:{guild_id}");

        let emoji_id = get_emoji_id(&emoji);

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

        db.is_connected().await?;

        let message_id = reaction.message_id;

        let table_id = format!("guild:{guild_id}");

        let emoji_id = get_emoji_id(&reaction.emoji);

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

        db.is_connected().await?;

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
            return Err("Bot reaction has been removed. This also removes the ability to get roles from the reaction.".into());
        }

        let mut member = guild_id.member(&ctx, user_id).await?;

        member.remove_role(&ctx, RoleId(role_id)).await?;

        Ok(())
    }
}

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
