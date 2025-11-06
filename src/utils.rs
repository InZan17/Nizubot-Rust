use std::{
    cell::Cell,
    collections::HashMap,
    hash::Hash,
    sync::Mutex,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use poise::serenity_prelude::{GuildId, UserId};

pub fn get_seconds() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards. Oopsie.");

    since_the_epoch.as_secs()
}
#[derive(Eq, Hash, PartialEq, Clone)]
pub enum IdType {
    UserId(UserId),
    GuildId(GuildId),
}

impl IdType {
    pub fn into_db_table(&self) -> String {
        match self {
            IdType::UserId(user_id) => format!("user:{user_id}"),
            IdType::GuildId(guild_id) => format!("guild:{guild_id}"),
        }
    }

    pub fn get_u64(&self) -> u64 {
        match self {
            IdType::UserId(id) => id.get(),
            IdType::GuildId(id) => id.get(),
        }
    }

    pub fn is_user(&self) -> bool {
        match self {
            IdType::UserId(_) => true,
            IdType::GuildId(_) => false,
        }
    }
}

pub struct TtlMap<K, V>
where
    K: Eq + Hash,
{
    map: HashMap<K, (V, Mutex<Instant>)>,
    max_lifetime: Duration,
}

impl<K, V> TtlMap<K, V>
where
    K: Eq + Hash,
{
    pub fn new(max_lifetime: Duration) -> TtlMap<K, V> {
        Self {
            map: HashMap::new(),
            max_lifetime,
        }
    }

    pub fn get(&self, k: &K) -> Option<&V> {
        self.map.get(k).map(|(value, last_accessed)| {
            *last_accessed.lock().unwrap() = Instant::now();
            value
        })
    }

    pub fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        self.map.get_mut(k).map(|(value, last_accessed)| {
            *last_accessed.lock().unwrap() = Instant::now();
            value
        })
    }

    pub fn insert(&mut self, k: K, v: V) {
        self.map.insert(k, (v, Mutex::new(Instant::now())));
    }

    pub fn contains_key(&self, k: &K) -> bool {
        self.map.contains_key(k)
    }

    pub fn clear_expired(&mut self) {
        if let Some(invalid_before) = Instant::now().checked_sub(self.max_lifetime) {
            self.map
                .retain(|_, (_, last_accessed)| *last_accessed.lock().unwrap() >= invalid_before);
        }
    }
}
