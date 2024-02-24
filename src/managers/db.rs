use std::{collections::HashMap, f32::consts::E, fmt::format};

use poise::serenity_prelude::{GuildId, MessageId, RoleId, UserId};
use reqwest::{Client, RequestBuilder};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

use crate::{
    tokens::{self, SurrealDbSignInInfo},
    utils::IdType,
    Error,
};

use super::{
    cotd_manager::{ColorInfo, CotdRoleData, CotdRoleDataQuery},
    detector_manager::DetectorInfo,
    remind_manager::RemindInfo,
};

pub struct SurrealClient {
    client: Client,
    sign_in_info: SurrealDbSignInInfo,
}

pub trait OptionOrVec {
    fn is_option() -> bool;
}

impl<T> OptionOrVec for Option<T> {
    fn is_option() -> bool {
        true
    }
}
impl<T> OptionOrVec for Vec<T> {
    fn is_option() -> bool {
        false
    }
}

#[derive(Deserialize)]
pub struct DbResponse {
    pub result: Value,
    pub status: String,
    pub time: String,
}

#[derive(Deserialize)]
pub struct DbError {
    pub code: u16,
    pub details: String,
    pub description: String,
    pub information: String,
}

pub struct Responses(Vec<DbResponse>);

impl Responses {
    /// Takes and parses response from Vec.
    ///
    /// Returns error if failed to deserialize or if response isn't OK.
    pub fn take<T: OptionOrVec + DeserializeOwned>(&self, index: usize) -> Result<T, Error> {
        if index >= self.0.len() {
            return Err("Index too high.".into());
        }

        let response = &self.0[index];

        if response.status != "OK".to_string() {
            let Some(result) = response.result.as_str() else {
                return Err("Database response status isn't OK.".into());
            };
            return Err(format!("Database response status isn't OK. {}", result).into());
        }

        let deserialize_value = if T::is_option() {
            value_option_fixer(&response.result)?
        } else {
            &response.result
        };

        let deserialized = serde_json::from_value::<T>(deserialize_value.clone())?;

        Ok(deserialized)
    }

    pub fn take_err(&self, index: usize) -> Option<Error> {
        if index >= self.0.len() {
            return Some("Index too high.".into());
        }

        let response = &self.0[index];

        if response.status != "OK".to_string() {
            let Some(result) = response.result.as_str() else {
                return Some("Database response status isn't OK.".into());
            };
            return Some(remove_prefix(result.to_string(), "An error occurred: ").into());
        }

        None
    }
}

fn remove_prefix(mut string: String, prefix: &str) -> String {
    if string.starts_with(prefix) {
        let prefix_length = prefix.len();
        string.replace_range(0..prefix_length, "");
    }
    string
}

impl SurrealClient {
    pub fn new(sign_in_info: SurrealDbSignInInfo) -> Self {
        let client = reqwest::Client::new();
        Self {
            client,
            sign_in_info,
        }
    }

    /// Creates the RequestBuilder with the url, auth and headers already setup.
    pub fn create_builder(&self) -> RequestBuilder {
        let sign_in = &self.sign_in_info;
        self.client
            .post(format!("{}/sql", &sign_in.address))
            .basic_auth(&sign_in.username, Some(&sign_in.password))
            .header("Accept", "application/json")
            .header("NS", &sign_in.namespace)
            .header("DB", &sign_in.database)
    }

    /// Sends query to database and returns the result.
    ///
    /// Errors if database is offline or if address is invalid.
    pub async fn query<S: Into<String>>(&self, query: S) -> Result<Responses, Error> {
        let query: String = query.into();
        let builder = self.create_builder().body(query.clone());

        let built_request = match builder.build() {
            Ok(request) => request,
            Err(err) => {
                println!("{err}");
                return Err("Failed to build request to database.".into());
            }
        };
        let response = match self.client.execute(built_request).await {
            Ok(response) => response,
            Err(err) => {
                println!("{err}");
                return Err("Failed to execute request to database.".into());
            }
        };

        if !response.status().is_success() {
            println!("Failed query: {}", query);
            match response.json::<DbError>().await {
                Ok(ok_err) => {
                    println!("{}", ok_err.information);
                    return Err(format!(
                        "Database failed to understand query. {}, {}",
                        ok_err.details, ok_err.description
                    )
                    .into());
                }
                Err(err) => {
                    return Err(format!(
                        "Database failed to understand query. Failed to parse error. {}",
                        err
                    )
                    .into())
                }
            }
        }

        let db_responses = response.json::<Vec<DbResponse>>().await?;

        Ok(Responses(db_responses))
    }
}

/// Returns value that can be deserialized using Option<T>.
/// Only does something if value is an array.
///
/// Errors if array has more than 1 element.
pub fn value_option_fixer(value: &Value) -> Result<&Value, Error> {
    let Some(array) = value.as_array() else {
        return Ok(value);
    };

    if array.len() == 1 {
        return Ok(&array[0]);
    } else if array.len() == 0 {
        return Ok(&Value::Null);
    } else {
        return Err("Couldn't parse Vec into Option because it has more than 1 elements.".into());
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StoredData {
    pub content: String,
}

impl SurrealClient {
    pub async fn get_single_data(&self) -> Result<Option<StoredData>, crate::Error> {
        let stored_data: Option<StoredData> =
            self.query("SELECT * FROM stored_data:1").await?.take(0)?;

        Ok(stored_data)
    }

    pub async fn update_single_data(&self, data: &StoredData) -> Result<(), crate::Error> {
        let data_json = serde_json::to_string(data)?;
        self.query(format!("UPDATE stored_data:1 CONTENT {data_json};"))
            .await?;

        Ok(())
    }

    pub async fn get_guild_cotd_role(
        &self,
        guild_id: &GuildId,
    ) -> Result<Option<CotdRoleDataQuery>, crate::Error> {
        //id is needed in the query because another function uses it and I dont wanna make another struct.
        let cotd_role_data: Option<CotdRoleDataQuery> = self
            .query(format!(
                "SELECT id, cotd_role FROM guild:{guild_id} WHERE cotd_role;"
            ))
            .await?
            .take(0)?;

        Ok(cotd_role_data)
    }

    /// Gets the id and cotd_role from every guild where cotd_role exists
    pub async fn get_all_guild_cotd_role(&self) -> Result<Vec<CotdRoleDataQuery>, crate::Error> {
        let cotd_roles_data: Vec<CotdRoleDataQuery> = self
            .query(format!("SELECT id, cotd_role FROM guild WHERE cotd_role;"))
            .await?
            .take(0)?;

        Ok(cotd_roles_data)
    }

    pub async fn update_guild_cotd_role(
        &self,
        cotd_role_data: &Option<CotdRoleData>,
        guild_id: &GuildId,
    ) -> Result<(), crate::Error> {
        let cotd_role_data_string = serde_json::to_string(&cotd_role_data)?;

        let cotd_role_data = self
            .query(format!(
                "UPDATE guild:{guild_id} SET cotd_role = {cotd_role_data_string};"
            ))
            .await?;

        Ok(())
    }

    pub async fn mark_cotd_role_updated(
        &self,
        guild_id: &GuildId,
        current_day: u64,
    ) -> Result<(), Error> {
        self.query(format!("UPDATE guild:{guild_id} MERGE {{ cotd_role: {{ day:{current_day} }} }} WHERE cotd_role;")).await?;
        Ok(())
    }

    pub async fn get_cotd(&self, day: u64) -> Result<Option<ColorInfo>, Error> {
        let day_color: Option<ColorInfo> = self
            .query(format!("SELECT * FROM cotd:{day};"))
            .await?
            .take(0)?;

        Ok(day_color)
    }

    pub async fn get_or_update_cotd(
        &self,
        day: u64,
        color: &ColorInfo,
    ) -> Result<ColorInfo, Error> {
        let color_json = serde_json::to_string(color)?;
        //1 returns None if it updated or Some if there already was a color.
        let opt: Option<ColorInfo> = self
            .query(format!(
                "
            LET $day_color = (SELECT * FROM cotd:{day});

            IF array::len($day_color) == 0 {{
                CREATE cotd:{day} CONTENT {color_json};
            }} ELSE {{
                RETURN $day_color[0]
            }}
            "
            ))
            .await?
            .take(1)?;

        if let Some(color_info) = opt {
            Ok(color_info)
        } else {
            Ok(color.clone())
        }
    }

    pub async fn get_all_message_detectors(
        &self,
        id: &IdType,
    ) -> Result<Option<Vec<DetectorInfo>>, Error> {
        let table_id = id.into_db_table();

        let res = self
            .query(format!(
                "SELECT VALUE message_detectors FROM {table_id} WHERE message_detectors"
            ))
            .await?
            .take(0)?;

        Ok(res)
    }

    pub async fn add_message_detector(
        &self,
        id: &IdType,
        detect_info: &DetectorInfo,
    ) -> Result<(), Error> {
        let table_id = id.into_db_table();

        let detect_info_json = serde_json::to_string(&detect_info)?;

        self.query(format!(
            "UPDATE {table_id} SET message_detectors += {detect_info_json}"
        ))
        .await?;

        Ok(())
    }

    pub async fn remove_message_detector(&self, id: &IdType, index: usize) -> Result<(), Error> {
        let table_id = id.into_db_table();

        let err = self
            .query(format!(
            "LET $detectors = (SELECT VALUE message_detectors FROM {table_id} WHERE message_detectors);

            IF array::len($detectors) == 0 {{
                THROW \"Index isn't valid.\";
            }} ELSE IF array::len($detectors[0]) <= {index} {{
                RETURN \"Index isn't valid.\";
            }} ELSE {{
                RETURN (UPDATE {table_id} SET message_detectors = array::remove(message_detectors, {index}));
            }};"
        ))
            .await?
            .take_err(1);

        if let Some(err) = err {
            return Err(err);
        }

        Ok(())
    }

    pub async fn get_message_reaction_roles(
        &self,
        guild_id: &GuildId,
        message_id: &MessageId,
    ) -> Result<Option<HashMap<String, RoleId>>, Error> {
        let res = self
            .query(format!(
                "SELECT VALUE messages.{message_id}.reaction_roles from guild:{guild_id};"
            ))
            .await?
            .take(0)?;

        Ok(res)
    }

    pub async fn clear_message_data(
        &self,
        id: &IdType,
        message_id: &MessageId,
    ) -> Result<(), Error> {
        let table_id = id.into_db_table();
        let res = self
            .query(format!(
                "UPDATE {table_id} SET messages.{message_id} = NONE;"
            ))
            .await?;

        if let Some(err) = res.take_err(0) {
            return Err(err);
        }

        Ok(())
    }

    pub async fn set_message_reaction_role(
        &self,
        guild_id: &GuildId,
        message_id: &MessageId,
        emoji_id: &str,
        role_id: Option<&RoleId>,
    ) -> Result<(), Error> {
        let role_id_format = if let Some(role_id) = role_id {
            format!("{role_id}")
        } else {
            "NONE".to_owned()
        };
        // I have to use merge here because if I try doing ["🧀"] like I do on the other queries then inside the database it will be "'🧀'" instead of "🧀"
        self.query(format!("UPDATE guild:{guild_id} MERGE {{ \"messages\": {{ {message_id}: {{ \"reaction_roles\": {{ \"{emoji_id}\": {role_id_format} }} }} }} }};")).await?;
        Ok(())
    }

    pub async fn get_role_from_message_reaction(
        &self,
        guild_id: &GuildId,
        message_id: &MessageId,
        emoji_id: &str,
    ) -> Result<Option<RoleId>, Error> {
        let role_id = self
            .query(format!(
                "SELECT VALUE messages.{message_id}.reaction_roles['{emoji_id}'] from guild:{guild_id};"
            ))
            .await?
            .take(0)?;

        Ok(role_id)
    }

    pub async fn list_user_reminders(&self, user_id: &UserId) -> Result<Vec<RemindInfo>, Error> {
        let user_reminders: Vec<RemindInfo> = self
            .query(format!(
                "
            LET $reminders = SELECT VALUE ->reminds->reminder FROM user:{user_id};

            IF array::len($reminders) THEN
                SELECT * FROM array::first($reminders) ORDER BY original_time;
            ELSE
                RETURN [];
            END
        "
            ))
            .await?
            .take(1)?;

        Ok(user_reminders)
    }

    pub async fn add_user_reminder(&self, remind_info: &RemindInfo) -> Result<(), Error> {
        let remind_info_json = serde_json::to_string(&remind_info)?;

        let user_id = remind_info.user_id;
        let guild_relate_statement = if let Some(guild_id) = remind_info.guild_id {
            format!(
                "
            UPDATE guild:{guild_id};
            RELATE guild:{guild_id}->reminds->$reminder;
            "
            )
        } else {
            "RETURN;RETURN;".to_owned()
        };

        self.query(format!(
            "
        BEGIN TRANSACTION;

        LET $reminder = (CREATE reminder CONTENT {remind_info_json});

        UPDATE user:{user_id};
        RELATE user:{user_id}->reminds->$reminder;

        {guild_relate_statement}

        COMMIT TRANSACTION;
        "
        ))
        .await?;

        Ok(())
    }

    pub async fn delete_table_id(&self, table_id: &String) -> Result<(), Error> {
        self.query(format!("DELETE {table_id}"))
            .await?
            .take::<Vec<Value>>(0)?;
        Ok(())
    }

    /// Returns a list of reminders that have finished and needs to be sent.
    pub async fn get_pending_reminders(&self, current_time: u64) -> Result<Vec<RemindInfo>, Error> {
        let reminders = self
            .query(format!(
                "SELECT * FROM reminder WHERE finish_time <= {current_time};"
            ))
            .await?
            .take(0)?;

        Ok(reminders)
    }
}
