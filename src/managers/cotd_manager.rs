use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use poise::serenity_prelude::{Context, Http, Role};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

use super::storage_manager::{DataDirectories, StorageManager};

pub const SECONDS_IN_A_DAY: u64 = 86400;
const COLOR_API: &str = "https://api.color.pizza/v1/";

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ColorResponse {
    colors: Vec<ColorInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct CotdRoleInfo {
    pub name: String,
    pub day: u64,
    pub id: u64,
}

pub struct CotdManager {
    storage_manager: Arc<StorageManager>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColorInfo {
    pub name: String,
    #[serde(alias = "color", alias = "hex")]
    pub hex: String,
}

impl CotdManager {
    pub fn new(storage_manager: Arc<StorageManager>) -> Self {
        Self { storage_manager }
    }

    pub async fn get_current_color(&self) -> Result<ColorInfo, String> {
        let current_day = self.get_current_day();
        return self.get_day_color(current_day).await;
    }

    pub fn get_current_day(&self) -> u64 {
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards. Oopsie.");

        since_the_epoch.as_secs() / SECONDS_IN_A_DAY
    }

    pub async fn get_day_color(&self, day: u64) -> Result<ColorInfo, String> {
        if day > self.get_current_day() {
            return Err("We have not reached that day yet.".to_owned());
        }

        let data = self
            .storage_manager
            .get_data_or_default::<HashMap<u64, ColorInfo>>(vec!["cotds"], HashMap::new())
            .await;

        let read = data.get_data().await;
        let color_info = read.get(&day).cloned();

        if let Some(color_info) = color_info {
            return Ok(color_info);
        }

        drop(read);

        match self.generate_color().await {
            Ok(color_info) => {
                let mut write = data.get_data_mut().await;
                write.insert(day, color_info.clone());
                data.request_file_write().await;
                return Ok(color_info);
            }
            Err(err) => return Err(err),
        }
    }

    pub async fn generate_color(&self) -> Result<ColorInfo, String> {
        const TWO_POW_24: u32 = 16777216;

        let random_color =
            poise::serenity_prelude::Colour::from(thread_rng().gen_range(0..TWO_POW_24));

        let Ok(resp) = reqwest::get(format!("{COLOR_API}?values={}", random_color.hex())).await else {
            return Err("Got no response from the Api.".to_owned());
        };

        let Ok(parsed) = resp.json::<ColorResponse>().await else {
            return Err("Couldn't parse Api response.".to_owned());
        };

        let mut color_info = parsed.colors[0].clone();
        color_info.hex.remove(0);

        Ok(color_info)
    }

    pub async fn update_role(
        &self,
        http: impl AsRef<Http>,
        role: Role,
        name: &String,
    ) -> Result<(), String> {
        match self.get_current_color().await {
            Err(err_text) => return Err(err_text),
            Ok(color_info) => {
                let res = role
                    .edit(http, |r| {
                        let color =
                            u64::from_str_radix(color_info.hex.clone().as_str(), 16).unwrap();
                        r.name(name.replace("<cotd>", &color_info.name))
                            .colour(color)
                    })
                    .await;

                match res {
                    Ok(_) => return Ok(()),
                    Err(err) => return Err(err.to_string()),
                }
            }
        }
    }
}

pub fn cotd_manager_loop(
    arc_ctx: Arc<Context>,
    storage_manager: Arc<StorageManager>,
    cotd_manager: Arc<CotdManager>,
) {
    tokio::spawn(async move {
        let mut last_updated_day = 0;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let current_day = cotd_manager.get_current_day();

            if last_updated_day == current_day {
                continue;
            }

            let cotd_roles_data = storage_manager
                .get_data_or_default::<Vec<u64>>(DataDirectories::cotd_guilds(), vec![])
                .await;

            last_updated_day = current_day;

            let mut removal = vec![];

            for guild_id in cotd_roles_data.get_data().await.iter() {
                let guild_cotd_role_data = storage_manager
                    .get_data::<CotdRoleInfo>(vec!["guilds", &guild_id.to_string(), "cotd_role"])
                    .await;

                let Some(guild_cotd_role_data) = guild_cotd_role_data else {
                    removal.push(*guild_id);
                    continue;
                };
                let mut cotd_role_data = guild_cotd_role_data.get_data_mut().await;

                if cotd_role_data.day == current_day {
                    continue;
                }

                let role;

                if let Some(guild) = arc_ctx.cache.guild(*guild_id) {
                    role = guild
                        .roles
                        .get(&poise::serenity_prelude::RoleId(cotd_role_data.id))
                        .cloned();
                } else {
                    let guild_res = arc_ctx.http.get_guild(*guild_id).await;

                    match guild_res {
                        Ok(guild) => {
                            role = guild
                                .roles
                                .get(&poise::serenity_prelude::RoleId(cotd_role_data.id))
                                .cloned();
                        }
                        Err(err) => {
                            println!("{}", err.to_string());
                            continue;
                        }
                    }
                }

                if let Some(role) = role {
                    let _result = cotd_manager
                        .update_role(&arc_ctx, role, &cotd_role_data.name)
                        .await;
                    cotd_role_data.day = current_day;
                    guild_cotd_role_data.request_file_write().await;
                } else {
                    removal.push(*guild_id);
                }
            }

            if removal.len() != 0 {
                cotd_roles_data
                    .get_data_mut()
                    .await
                    .retain(|guild_id| !removal.contains(guild_id));
                cotd_roles_data.request_file_write().await;
            }
        }
    });
}
