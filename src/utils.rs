use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex},
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
#[derive(Eq, Hash, PartialEq, Clone, Copy)]
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

pub struct TtlMapWithArcTokioMutex<K, V>
where
    K: Eq + Hash,
{
    map: HashMap<K, (Arc<tokio::sync::Mutex<V>>, Mutex<Option<Instant>>)>,
    max_lifetime: Duration,
}

impl<K, V> TtlMapWithArcTokioMutex<K, V>
where
    K: Eq + Hash,
{
    pub fn new(max_lifetime: Duration) -> TtlMapWithArcTokioMutex<K, V> {
        Self {
            map: HashMap::new(),
            max_lifetime,
        }
    }

    pub fn get_raw_map(
        &mut self,
    ) -> &mut HashMap<K, (Arc<tokio::sync::Mutex<V>>, Mutex<Option<Instant>>)> {
        &mut self.map
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn get(&self, k: &K) -> Option<Arc<tokio::sync::Mutex<V>>> {
        self.map.get(k).map(|(value, last_accessed)| {
            *last_accessed.lock().unwrap() = None;
            value.clone()
        })
    }

    pub fn insert(&mut self, k: K, v: V) -> Arc<tokio::sync::Mutex<V>> {
        let value = Arc::new(tokio::sync::Mutex::new(v));
        self.map
            .insert(k, (value.clone(), Mutex::new(Some(Instant::now()))));
        value
    }

    pub fn contains_key(&self, k: &K) -> bool {
        self.map.contains_key(k)
    }

    pub fn clear_expired(&mut self) {
        if let Some(invalid_before) = Instant::now().checked_sub(self.max_lifetime) {
            self.map.retain(|_, (v, last_accessed)| {
                let mut lock = last_accessed.lock().unwrap();

                if let Some(instant) = *last_accessed.lock().unwrap() {
                    return instant >= invalid_before;
                };

                if Arc::strong_count(v) > 1 {
                    return true;
                }

                *lock = Some(Instant::now());
                return true;
            });
        }
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }
}
