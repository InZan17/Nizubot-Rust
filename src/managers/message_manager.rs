use std::collections::HashMap;

use poise::serenity_prelude::{ChannelId, MessageId, RoleId};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct StoredMessageData {
    pub message_id: Option<MessageId>,
    pub channel_id: Option<ChannelId>,
    #[serde(default)]
    pub reaction_roles: HashMap<String, RoleId>,
}

impl StoredMessageData {
    pub fn needs_updating(&self) -> bool {
        self.message_id.is_none() || self.channel_id.is_none()
    }
}

// TODO: A manager to house cached messages.
