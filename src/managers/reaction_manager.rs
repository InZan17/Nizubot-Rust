use std::{collections::HashMap, sync::Arc, time::Duration};

use poise::serenity_prelude::{
    self, ChannelId, Context, GuildId, MessageId, Reaction, ReactionType, RoleId, UserId,
};
use tokio::sync::{Mutex, RwLock};

use crate::{utils::TtlMap, Error};

use super::{db::SurrealClient, message_manager::StoredMessageData};

pub struct MessagesData {
    pub guild_id: GuildId,
    pub messages: Option<HashMap<MessageId, StoredMessageData>>,
}

impl MessagesData {
    pub fn new(guild_id: GuildId) -> Self {
        Self {
            guild_id,
            messages: None,
        }
    }

    pub async fn get_messages(
        &mut self,
        db: &SurrealClient,
    ) -> Result<&mut HashMap<MessageId, StoredMessageData>, Error> {
        let messages_mut = &mut self.messages;
        match messages_mut {
            Some(messages) => return Ok(messages),
            None => {
                let fetched_messages = db.get_guild_messages(self.guild_id).await?;

                *messages_mut = Some(fetched_messages);
                return Ok(messages_mut.as_mut().unwrap());
            }
        }
    }

    pub async fn add_or_replace_message(
        &mut self,
        message_id: MessageId,
        message_data: StoredMessageData,
        db: &SurrealClient,
    ) -> Result<(), Error> {
        let guild_id = self.guild_id;
        let messages = self.get_messages(db).await?;
        db.set_guild_message(guild_id, message_id, &message_data)
            .await?;
        messages.insert(message_id, message_data);
        Ok(())
    }

    pub async fn delete_message(
        &mut self,
        message_id: MessageId,
        db: &SurrealClient,
    ) -> Result<(), Error> {
        let guild_id = self.guild_id;
        let messages = self.get_messages(db).await?;
        db.remove_guild_message(guild_id, message_id).await?;
        messages.remove(&message_id);
        Ok(())
    }
}

pub struct ReactionManager {
    pub db: Arc<SurrealClient>,
    pub messages_data: RwLock<TtlMap<GuildId, Arc<Mutex<MessagesData>>>>,
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

impl ReactionTypeOrRoleId {
    pub fn is_reaction(&self) -> bool {
        match self {
            ReactionTypeOrRoleId::ReactionType(_) => true,
            ReactionTypeOrRoleId::RoleId(_) => false,
        }
    }
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
        Self {
            db,
            messages_data: RwLock::new(TtlMap::new(Duration::from_secs(60 * 60))),
        }
    }

    /// NOTE: It is VERY IMPORTANT that you do not store this Arc anywhere for long term use!
    pub async fn get_messages_data(&self, guild_id: GuildId) -> Arc<Mutex<MessagesData>> {
        if let Some(messages_data) = self.messages_data.read().await.get(&guild_id).cloned() {
            return messages_data;
        }

        let mut messages_data_mut = self.messages_data.write().await;
        if let Some(messages_data) = messages_data_mut.get(&guild_id).cloned() {
            return messages_data;
        }

        let messages_data = Arc::new(Mutex::new(MessagesData::new(guild_id)));

        messages_data_mut.insert(guild_id, messages_data.clone());

        messages_data
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

        let messages_data = self.get_messages_data(guild_id).await;
        let mut locked_messages_data = messages_data.lock().await;

        let message_data_option = locked_messages_data
            .get_messages(db)
            .await
            .map_err(|err| {
                ReactionError::Database(
                    err,
                    "Couldn't fetch current reaction roles from database.".to_string(),
                )
            })?
            .get(&message_id)
            .cloned();

        let mut message_data;

        if let Some(some_message_data) = message_data_option {
            message_data = some_message_data;
        } else {
            message_data = StoredMessageData {
                channel_id: Some(channel_id),
                reaction_roles: HashMap::new(),
            }
        }

        if message_data.needs_updating() {
            message_data.channel_id = Some(channel_id);
        }

        if let Some(role_id) = message_data.reaction_roles.get(&get_emoji_id(&emoji)) {
            return Err(ReactionError::EmojiTaken(*role_id));
        }

        // Check if role is already registered on the same message.
        for (other_emoji, other_role_id) in message_data.reaction_roles.iter() {
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

        message_data.reaction_roles.insert(emoji_id, role_id);

        if let Err(err) = locked_messages_data
            .add_or_replace_message(message_id, message_data, db)
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
    pub async fn remove_reaction_role(
        &self,
        emoji_or_role: ReactionTypeOrRoleId,
        guild_id: GuildId,
        message_id: MessageId,
    ) -> Result<RoleId, ReactionError> {
        let db = &self.db;

        let messages_data = self.get_messages_data(guild_id).await;
        let mut locked_messages_data = messages_data.lock().await;

        let message_data_option = locked_messages_data
            .get_messages(db)
            .await
            .map_err(|err| {
                ReactionError::Database(
                    err,
                    "Couldn't fetch current reaction roles from database.".to_string(),
                )
            })?
            .get(&message_id)
            .cloned();

        let Some(mut message_data) = message_data_option else {
            if emoji_or_role.is_reaction() {
                return Err(ReactionError::NoReaction);
            } else {
                return Err(ReactionError::NoRole);
            }
        };

        let emoji_id_removal;

        match emoji_or_role {
            ReactionTypeOrRoleId::ReactionType(emoji) => {
                emoji_id_removal = get_emoji_id(&emoji);
            }
            ReactionTypeOrRoleId::RoleId(role_id) => {
                emoji_id_removal = message_data
                    .reaction_roles
                    .iter()
                    .find(|(_k, v)| **v == role_id)
                    .map(|(k, _v)| k.clone())
                    .unwrap_or_default();

                if emoji_id_removal.is_empty() {
                    return Err(ReactionError::NoReaction);
                }
            }
        }

        let role_id = message_data.reaction_roles.remove(&emoji_id_removal);

        let Some(role_id) = role_id else {
            return Err(ReactionError::NoReaction);
        };

        if message_data.reaction_roles.is_empty() {
            locked_messages_data
                .delete_message(message_id, db)
                .await
                .map_err(|err| {
                    ReactionError::Database(
                        err,
                        "Couldn't remove reaction role from message.".to_string(),
                    )
                })?;
        } else {
            locked_messages_data
                .add_or_replace_message(message_id, message_data, db)
                .await
                .map_err(|err| {
                    ReactionError::Database(
                        err,
                        "Couldn't remove reaction role from message.".to_string(),
                    )
                })?;
        }

        Ok(role_id)
    }

    /// Removes all reaction events to a message.
    ///
    /// Errors if communication with db doesn't work or if there's no emoji registered for that message.
    pub async fn remove_all_reaction_roles(
        &self,
        guild_id: GuildId,
        message_id: MessageId,
    ) -> Result<(), ReactionError> {
        let db = &self.db;

        let messages_data = self.get_messages_data(guild_id).await;
        let mut locked_messages_data = messages_data.lock().await;

        let message_data_option = locked_messages_data
            .get_messages(db)
            .await
            .map_err(|err| {
                ReactionError::Database(
                    err,
                    "Couldn't fetch current reaction roles from database.".to_string(),
                )
            })?
            .get(&message_id);

        let Some(message_data) = message_data_option else {
            return Ok(());
        };

        if message_data.reaction_roles.is_empty() {
            return Ok(());
        }

        locked_messages_data
            .delete_message(message_id, db)
            .await
            .map_err(|err| {
                ReactionError::Database(
                    err,
                    "Couldn't remove reaction roles from message.".to_string(),
                )
            })?;

        Ok(())
    }

    /// Gets all reaction roles given a guild id and message id.
    pub async fn get_reaction_roles(
        &self,
        guild_id: GuildId,
        message_id: MessageId,
    ) -> Result<HashMap<String, RoleId>, ReactionError> {
        let db = &self.db;

        let messages_data = self.get_messages_data(guild_id).await;
        let mut locked_messages_data = messages_data.lock().await;

        let message_data_option = locked_messages_data
            .get_messages(db)
            .await
            .map_err(|err| {
                ReactionError::Database(
                    err,
                    "Couldn't fetch current reaction roles from database.".to_string(),
                )
            })?
            .get(&message_id)
            .cloned();

        let reaction_roles = message_data_option
            .map(|v| v.reaction_roles)
            .unwrap_or_default();

        Ok(reaction_roles)
    }

    /// Gets all reaction role messages in a guild and the amount of reaction roles on them.
    pub async fn get_reaction_role_messages(
        &self,
        guild_id: GuildId,
    ) -> Result<Vec<(MessageId, Option<ChannelId>, usize)>, ReactionError> {
        let db = &self.db;

        let messages_data = self.get_messages_data(guild_id).await;
        let mut locked_messages_data = messages_data.lock().await;

        let messages_data = locked_messages_data.get_messages(db).await.map_err(|err| {
            ReactionError::Database(
                err,
                "Couldn't fetch current reaction roles from database.".to_string(),
            )
        })?;

        let mut mapped_messages = messages_data
            .iter()
            .filter(|(_, v)| v.reaction_roles.len() != 0)
            .map(|(k, v)| (*k, v.channel_id, v.reaction_roles.len()))
            .collect::<Vec<_>>();

        mapped_messages.sort();

        Ok(mapped_messages)
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

        let message_id = reaction.message_id;

        let emoji_id = get_emoji_id(&reaction.emoji);

        let db = &self.db;

        let messages_data = self.get_messages_data(guild_id).await;
        let mut locked_messages_data = messages_data.lock().await;

        let guild_messages = locked_messages_data.get_messages(db).await.map_err(|err| {
            ReactionError::Database(
                err,
                "Couldn't fetch current reaction roles from database.".to_string(),
            )
        })?;

        let Some(guild_message) = guild_messages.get(&message_id) else {
            return Ok(());
        };

        let Some(role_id) = guild_message.reaction_roles.get(&emoji_id).copied() else {
            return Ok(());
        };

        if guild_message.needs_updating() {
            let mut guild_message = guild_message.clone();
            guild_message.channel_id = Some(reaction.channel_id);
            let _ = locked_messages_data
                .add_or_replace_message(message_id, guild_message, db)
                .await;
        }

        let member = match guild_id.member(&ctx, user_id).await {
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

    /// Runs whenever a user un-reacts to a message.
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

        let messages_data = self.get_messages_data(guild_id).await;
        let mut locked_messages_data = messages_data.lock().await;

        let guild_messages = locked_messages_data.get_messages(db).await.map_err(|err| {
            ReactionError::Database(
                err,
                "Couldn't fetch current reaction roles from database.".to_string(),
            )
        })?;

        let Some(guild_message) = guild_messages.get(&message_id) else {
            return Ok(());
        };

        let Some(role_id) = guild_message.reaction_roles.get(&emoji_id).copied() else {
            return Ok(());
        };

        if user_id == bot_id {
            let mut guild_message = guild_message.clone();
            let Some(removed_role_id) = guild_message.reaction_roles.remove(&emoji_id) else {
                return Ok(());
            };

            if let Err(err) = locked_messages_data
                .add_or_replace_message(message_id, guild_message, db)
                .await
            {
                return Err(ReactionError::Database(
                    err,
                    "Bot un-reacted to reaction but couldn't remove reaction role from the message in the database."
                        .to_string(),
                ));
            };
            return Err(ReactionError::BotReactionRemoved(removed_role_id, emoji_id));
        } else if guild_message.needs_updating() {
            let mut guild_message = guild_message.clone();
            guild_message.channel_id = Some(reaction.channel_id);
            let _ = locked_messages_data
                .add_or_replace_message(message_id, guild_message, db)
                .await;
        }

        let member = match guild_id.member(&ctx, user_id).await {
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

    pub async fn reaction_remove_all(
        &self,
        guild_id: GuildId,
        message_id: MessageId,
    ) -> Result<(), ReactionError> {
        let db = &self.db;

        let messages_data = self.get_messages_data(guild_id).await;
        let mut locked_messages_data = messages_data.lock().await;

        let guild_messages = locked_messages_data.get_messages(db).await.map_err(|err| {
            ReactionError::Database(
                err,
                "Couldn't fetch current reaction roles from database.".to_string(),
            )
        })?;

        if !guild_messages.contains_key(&message_id) {
            return Ok(());
        };

        if let Err(err) = locked_messages_data.delete_message(message_id, db).await {
            return Err(ReactionError::Database(
                    err,
                    "A message got all of its reactions removed, but I couldn't remove the reaction roles from the message in the database."
                        .to_string(),
                ));
        };

        return Ok(());
    }

    pub async fn reaction_remove_emoji(&self, reaction: &Reaction) -> Result<(), ReactionError> {
        let Some(guild_id) = reaction.guild_id else {
            return Ok(());
        };

        let message_id = reaction.message_id;

        let emoji_id = get_emoji_id(&reaction.emoji);

        let db = &self.db;

        let messages_data = self.get_messages_data(guild_id).await;
        let mut locked_messages_data = messages_data.lock().await;

        let guild_messages = locked_messages_data.get_messages(db).await.map_err(|err| {
            ReactionError::Database(
                err,
                "Couldn't fetch current reaction roles from database.".to_string(),
            )
        })?;

        let Some(mut guild_message) = guild_messages.get(&message_id).cloned() else {
            return Ok(());
        };

        if guild_message.reaction_roles.remove(&emoji_id).is_none()
            && !guild_message.needs_updating()
        {
            return Ok(());
        }

        if guild_message.needs_updating() {
            guild_message.channel_id = Some(reaction.channel_id)
        }

        if let Err(err) = locked_messages_data
            .add_or_replace_message(message_id, guild_message, db)
            .await
        {
            return Err(ReactionError::Database(
                    err,
                    "Bot un-reacted to reaction but couldn't remove reaction role from the message in the database."
                        .to_string(),
                ));
        };

        return Ok(());
    }
}

pub fn reaction_manager_loop(reaction_manager: Arc<ReactionManager>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30 * 60)).await;
            let mut messages_data_write = reaction_manager.messages_data.write().await;
            messages_data_write.clear_expired();
        }
    });
}

/// Returns the id of a custom emoji, not including its name.
/// If it's an unicode emoji it will return the unicode emoji.
fn get_emoji_id(emoji: &ReactionType) -> String {
    match emoji {
        ReactionType::Custom {
            animated: _,
            id,
            name: _,
        } => id.get().to_string(),
        ReactionType::Unicode(name) => name.to_string(),
        _ => emoji.as_data(),
    }
}
