use std::collections::HashMap;

use poise::serenity_prelude::{ChannelId, RoleId};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct StoredMessageData {
    // MessageId is not stored because, most likely, this is stored in a map where the MessageId is the key.
    pub channel_id: Option<ChannelId>,
    #[serde(default)]
    pub reaction_roles: HashMap<String, RoleId>,
}

impl StoredMessageData {
    /// Checks if the data is from an old version of Nizubot where the ChannelId wasn't being stored.
    pub fn needs_updating(&self) -> bool {
        self.channel_id.is_none()
    }
}
