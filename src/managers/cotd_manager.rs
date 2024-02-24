use std::{
    collections::HashMap,
    fmt::format,
    sync::Arc,
    thread::current,
    time::{SystemTime, UNIX_EPOCH},
};

use poise::serenity_prelude::{self, guild, Context, GuildId, Http, Role, RoleId};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};

use crate::{managers::remind_manager::is_user_fault, utils::IdType, Error};

use super::{
    db::SurrealClient,
    log_manager::{LogManager, LogSource, LogType},
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
    pub id: RoleId,
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
pub enum CotdError {
    UnreachedDay,
    Serenity(serenity_prelude::Error, String),
    Other(Error, String),
}

impl CotdError {
    pub fn to_string(&self) -> String {
        match self {
            CotdError::UnreachedDay => "We have not reached that day yet.".to_string(),
            CotdError::Serenity(err, description) => format!("{description} {err}"),
            CotdError::Other(err, description) => format!("{description} {err}"),
        }
    }

    pub fn is_user_fault(&self) -> bool {
        match self {
            CotdError::UnreachedDay => false,
            CotdError::Serenity(err, _) => is_user_fault(err),
            CotdError::Other(_, _) => false,
        }
    }
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
    pub async fn get_current_color(&self) -> Result<ColorInfo, CotdError> {
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
    pub async fn get_day_color(&self, day: u64) -> Result<ColorInfo, CotdError> {
        let is_future_day = day > self.get_current_day();
        if is_future_day {
            return Err(CotdError::UnreachedDay);
        }

        let color = match self.db.get_cotd(day).await {
            Ok(ok) => ok,
            Err(err) => {
                return Err(CotdError::Other(
                    err,
                    "Couldn't get day color from database.".to_string(),
                ))
            }
        };

        if let Some(color) = color {
            //return Ok(color);
        }

        match self.generate_color().await {
            Ok(color_info) => match self.db.get_or_update_cotd(day, &color_info).await {
                Ok(color_info) => Ok(color_info),
                Err(err) => {
                    return Err(CotdError::Other(
                        err,
                        "Couldn't update day color from database.".to_string(),
                    ))
                }
            },
            Err(err) => return Err(err),
        }
    }

    /// Generates a pseudo random color.
    ///
    /// Will error if it cannot connect to COLOR_API since it's used to get the name of the color.
    pub async fn generate_color(&self) -> Result<ColorInfo, CotdError> {
        const TWO_POW_24: u32 = 16777216;

        let random_color =
            poise::serenity_prelude::Colour::from(thread_rng().gen_range(0..TWO_POW_24));

        let url = format!("{COLOR_API}?values={}", random_color.hex());

        let resp = match reqwest::get(url).await {
            Ok(resp) => resp,
            Err(err) => {
                return Err(CotdError::Other(
                    err.into(),
                    "Couldn't generate color name. Got no response from color api.".to_string(),
                ))
            }
        };

        let parsed = match resp.json::<ColorResponse>().await {
            Ok(parsed) => parsed,
            Err(err) => {
                return Err(CotdError::Other(
                    err.into(),
                    "Couldn't generate color name. Couldn't parse response from color api."
                        .to_string(),
                ))
            }
        };

        let mut color_info = parsed.colors[0].clone();
        color_info.hex.remove(0);

        Ok(color_info)
    }

    /// Updates the color of a given role.
    ///
    /// Will error if it cannot update the role.
    pub async fn update_role(
        &self,
        http: impl AsRef<Http>,
        guild_id: GuildId,
        role_id: RoleId,
        name: &String,
        current_color: &ColorInfo,
    ) -> Result<(), CotdError> {
        let res = guild_id
            .edit_role(http, role_id, |r| {
                let color = u64::from_str_radix(current_color.hex.clone().as_str(), 16).unwrap();
                r.name(name.replace("<cotd>", &current_color.name))
                    .colour(color)
            })
            .await;

        match res {
            Ok(_) => return Ok(()),
            Err(err) => {
                return Err(CotdError::Serenity(
                    err,
                    "Couldn't update cotd role.".to_string(),
                ))
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
    log_manager: Arc<LogManager>,
) {
    tokio::spawn(async move {
        let mut last_updated_day = 0;
        let mut cotd_is_late = false;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let current_day = cotd_manager.get_current_day();

            if last_updated_day == current_day {
                continue;
            }

            let cotd_roles_data_result = db.get_all_guild_cotd_role().await;

            let Ok(cotd_roles_data) = cotd_roles_data_result else {
                //We dont got any guilds ids. Cannot do anything right now.
                //Once we got access to it again, apologize to all guilds in their logs.
                cotd_is_late = true;
                continue;
            };

            let current_color_res = cotd_manager.get_current_color().await;

            let mut successful = true;

            for cotd_role_data_query in cotd_roles_data.iter() {
                let table_id = &cotd_role_data_query.id;

                let Some(guild_id_string) = table_id.split(':').last() else {
                    continue;
                };
                let Ok(guild_id_u64) = guild_id_string.parse::<u64>() else {
                    continue;
                };

                let guild_id = GuildId(guild_id_u64);
                let cotd_role_data = &cotd_role_data_query.cotd_role;

                if cotd_role_data.day == current_day {
                    continue;
                }

                if cotd_is_late {
                    let _ = log_manager.add_log(
                        &IdType::GuildId(guild_id),
                        "The cotd role update was late due to issues talking to database. I apologize.".to_string(),
                        LogType::Warning,
                        LogSource::CotdRole,
                    ).await;
                }

                let current_color = match &current_color_res {
                    Ok(ok) => ok,
                    Err(err) => {
                        let _ = log_manager
                            .add_log(
                                &IdType::GuildId(guild_id),
                                format!("Couldn't update cotd role. {}", err.to_string()),
                                LogType::Warning,
                                LogSource::CotdRole,
                            )
                            .await;
                        successful = false;
                        continue;
                    }
                };

                let result = cotd_manager
                    .update_role(
                        &arc_ctx,
                        guild_id,
                        cotd_role_data.id,
                        &cotd_role_data.name,
                        &current_color,
                    )
                    .await;

                if let Err(err) = result {
                    let _ = log_manager
                        .add_log(
                            &IdType::GuildId(guild_id),
                            err.to_string(),
                            LogType::Warning,
                            LogSource::CotdRole,
                        )
                        .await;

                    //TODO: Try to see if you can see what exactly errors. For example if it errored because the role is gone.
                    if !err.is_user_fault() {
                        successful = false;
                    }

                    continue;
                }

                db.mark_cotd_role_updated(&guild_id, current_day).await;
            }

            if successful {
                last_updated_day = current_day;
            }

            cotd_is_late = false;
        }
    });
}
