use std::{
    collections::HashMap,
    fmt::format,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use poise::serenity_prelude::{Context, Http, Role};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};

use crate::Error;

use super::{
    db::SurrealClient,
    storage_manager::{DataDirectories, StorageManager},
};

pub const SECONDS_IN_A_DAY: u64 = 86400;
const COLOR_API: &str = "https://api.color.pizza/v1/";

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ColorResponse {
    colors: Vec<ColorInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct CotdRoleData {
    pub name: String,
    pub day: u64,
    pub id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct CotdRoleDataQuery {
    pub cotd_role: CotdRoleData,
    pub id: String,
}

pub struct CotdManager {
    db: Arc<SurrealClient>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColorInfo {
    pub name: String,
    #[serde(alias = "color", alias = "hex")]
    pub hex: String,
}

impl CotdManager {
    pub fn new(db: Arc<SurrealClient>) -> Self {
        Self { db }
    }

    /// Gets the current color of the current day.
    ///
    /// Equivalent of calling self.get_day_color(self.get_current_day())
    ///
    /// Errors will happen if there's no connection between the bot and database or if connection to COLOR_API fails or if that day hasn't happened yet.
    pub async fn get_current_color(&self) -> Result<ColorInfo, Error> {
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

    /// Gets the color of a certain day.
    ///
    /// Errors will happen if there's no connection between the bot and database or if connection to COLOR_API fails or if that day hasn't happened yet.
    pub async fn get_day_color(&self, day: u64) -> Result<ColorInfo, Error> {
        let is_future_day = day > self.get_current_day();
        if is_future_day {
            return Err("We have not reached that day yet.".into());
        }

        // TODO: put this in a function
        let table_id = format!("cotd:{day}");
        // this too
        let day_color: Vec<ColorInfo> = self
            .db
            .query(format!("SELECT * FROM {table_id};"))
            .await?
            .take(0)?;

        if day_color.len() > 1 {
            return Err(format!("{table_id} returns more than 1 color.").into());
        }

        if day_color.len() == 1 {
            return Ok(day_color[0].clone());
        }

        // Since we will be generating colors every day even if no one runs a command
        // Let the looped code do that. And if the current day color doesnt exist the return an error.
        match self.generate_color().await {
            Ok(color_info) => {
                let color_info_string = serde_json::to_string(&color_info)?;
                let mut res = self
                    .db
                    .query(format!(
                        "
                    CREATE {table_id} CONTENT {color_info_string}; 
                    SELECT * FROM ONLY {table_id};
                "
                    ))
                    .await?;

                let Some(color_info) = res.take(1)? else {
                    return Err("Couldn't generate color. Problems with database and stuff.".into());
                };

                return Ok(color_info);
            }
            Err(err) => return Err(err),
        }
    }

    /// Generates a pseudo random color.
    ///
    /// Will error if it cannot connect to COLOR_API since it's used to get the name of the color.
    pub async fn generate_color(&self) -> Result<ColorInfo, Error> {
        const TWO_POW_24: u32 = 16777216;

        let random_color =
            poise::serenity_prelude::Colour::from(thread_rng().gen_range(0..TWO_POW_24));

        let url = format!("{COLOR_API}?values={}", random_color.hex());

        let Ok(resp) = reqwest::get(url).await else {
            //TODO: return error info
            return Err("Got no response from the Api.".into());
        };

        let Ok(parsed) = resp.json::<ColorResponse>().await else {
            //TODO: return error info
            return Err("Couldn't parse Api response.".into());
        };

        let mut color_info = parsed.colors[0].clone();
        color_info.hex.remove(0);

        Ok(color_info)
    }

    /// Updates the color of a given role.
    ///
    /// Will error if it cannot update the role or if getting the current color fails.
    pub async fn update_role(
        &self,
        http: impl AsRef<Http>,
        role: Role,
        name: &String,
    ) -> Result<(), Error> {
        //TODO: Perhaps get the color info from a function parameter. That way we dont have to constantly do a database request every time.
        match self.get_current_color().await {
            Err(err) => return Err(err),
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
                    Err(err) => return Err(Box::new(err)),
                }
            }
        }
    }
}
/// Loop that controls the cotd manager.
///
/// Controls things such as: the color of all cotd roles.
pub fn cotd_manager_loop(
    arc_ctx: Arc<Context>,
    db: Arc<SurrealClient>,
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

            //TODO: convert all cotd code to use surrealdb instead.
            // Gets the id and cotd_role from every guild where cotd_role exists
            let cotd_roles_data: Vec<CotdRoleDataQuery> = db
                .query("SELECT id, cotd_role FROM guild WHERE cotd_role;")
                .await
                .unwrap()
                .take(0)
                .unwrap();

            //TODO: Make it so it only updates when all the cotd roles updates successfully.
            // One way to check is if the length of cotd_roles_data is 0
            last_updated_day = current_day;

            for cotd_role_data_query in cotd_roles_data.iter() {
                let table_id = &cotd_role_data_query.id;
                let guild_id = table_id.split(' ').last().unwrap().parse::<u64>().unwrap(); //TODO fix too many unwraps
                let cotd_role_data = &cotd_role_data_query.cotd_role;

                if cotd_role_data.day == current_day {
                    continue;
                }

                let role;

                //TODO: Put this in a seperate function
                if let Some(guild) = arc_ctx.cache.guild(guild_id) {
                    role = guild
                        .roles
                        .get(&poise::serenity_prelude::RoleId(cotd_role_data.id))
                        .cloned();
                } else {
                    let guild_res = arc_ctx.http.get_guild(guild_id).await;

                    match guild_res {
                        Ok(guild) => {
                            role = guild
                                .roles
                                .get(&poise::serenity_prelude::RoleId(cotd_role_data.id))
                                .cloned();
                        }
                        Err(err) => {
                            //TODO: check if error is internet fault or user fault.
                            println!("{}", err.to_string());
                            continue;
                        }
                    }
                }

                if let Some(role) = role {
                    //TODO: Do something if there's an error.
                    let result = cotd_manager
                        .update_role(&arc_ctx, role, &cotd_role_data.name)
                        .await;

                    // TODO: Put query in seperate function
                    db.query(format!("UPDATE {table_id} MERGE {{ cotd_role: {{ day:{current_day} }} }} WHERE cotd_role;")).await;
                } else {
                    //TODO: role doesnt exist. Notify server error log and remove the role.
                }
            }
        }
    });
}
