use std::{clone, sync::Arc, time::Duration};

use chrono_tz::Tz;
use poise::serenity_prelude::UserId;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};

use crate::{
    commands::utility_commands::profile::time_format::TimeFormat, managers::db::SurrealClient,
    utils::TtlMap, Error,
};

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ProfileData {
    pub timezone: Option<String>,
    pub time_format: Option<TimeFormat>,
}

impl ProfileData {
    pub fn get_timezone(&self) -> Option<(String, Option<Tz>)> {
        self.timezone.clone().map(|timezone_name| {
            let timezone = Tz::from_str_insensitive(&timezone_name).ok();
            (timezone_name, timezone)
        })
    }

    pub fn get_time_format_with_fallback(&self, locale: &str) -> TimeFormat {
        self.time_format
            .unwrap_or_else(|| match locale.to_ascii_lowercase().as_str() {
                "en-us" | "hi" | "zh-tw" | "ko" => TimeFormat::Twelve,
                _ => TimeFormat::TwentyFour,
            })
    }
}

pub struct ProfileDataHolder {
    pub user_id: UserId,
    pub profile: Option<ProfileData>,
}

impl ProfileDataHolder {
    pub fn new(user_id: UserId) -> Self {
        Self {
            user_id,
            profile: None,
        }
    }

    pub async fn get_profile(&mut self, db: &SurrealClient) -> Result<&mut ProfileData, Error> {
        let profile_mut = &mut self.profile;
        match profile_mut {
            Some(profile) => return Ok(profile),
            None => {
                let fetched_profile = db.get_user_profile(self.user_id).await?;

                *profile_mut = Some(fetched_profile);
                return Ok(profile_mut.as_mut().unwrap());
            }
        }
    }

    pub async fn update_profile(
        &mut self,
        profile: ProfileData,
        db: &SurrealClient,
    ) -> Result<(), Error> {
        let user_id = self.user_id;

        let mut_profile = self.get_profile(db).await?;
        db.set_user_profile(user_id, &profile).await?;
        *mut_profile = profile;
        Ok(())
    }

    pub async fn delete_profile(&mut self, db: &SurrealClient) -> Result<(), Error> {
        db.delete_user_profile(self.user_id).await?;
        self.profile = Some(ProfileData::default());
        Ok(())
    }
}

pub struct ProfileManager {
    db: Arc<SurrealClient>,
    profiles: RwLock<TtlMap<UserId, Arc<Mutex<ProfileDataHolder>>>>,
}

impl ProfileManager {
    pub fn new(db: Arc<SurrealClient>) -> Self {
        Self {
            db,
            profiles: RwLock::new(TtlMap::new(Duration::from_secs(60 * 60))),
        }
    }

    pub async fn get_profile_data(&self, user_id: UserId) -> Arc<Mutex<ProfileDataHolder>> {
        if let Some(profile_data) = self.profiles.read().await.get(&user_id).cloned() {
            return profile_data;
        }

        let mut profile_data_mut = self.profiles.write().await;
        if let Some(profile_data) = profile_data_mut.get(&user_id).cloned() {
            return profile_data;
        }

        let profile_data = Arc::new(Mutex::new(ProfileDataHolder::new(user_id)));

        profile_data_mut.insert(user_id, profile_data.clone());

        profile_data
    }
}

pub fn profile_manager_loop(profile_manager: Arc<ProfileManager>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30 * 60)).await;
            let mut profiles_write = profile_manager.profiles.write().await;
            profiles_write.clear_expired();
        }
    });
}
