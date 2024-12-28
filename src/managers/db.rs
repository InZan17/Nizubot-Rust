use std::collections::HashMap;

use poise::serenity_prelude::{GuildId, MessageId, UserId};
use reqwest::{Client, RequestBuilder};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

use crate::{
    commands::utility_commands::time_format::TimeFormat, tokens::SurrealDbSignInInfo,
    utils::IdType, Error,
};

use super::{
    cotd_manager::{ColorInfo, CotdRoleData, CotdRoleDataQuery},
    detector_manager::DetectorInfo,
    lua_manager::LuaCommandInfo,
    message_manager::StoredMessageData,
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
                return Err(format!("Failed to build request to database. {err}").into());
            }
        };
        let response = match self.client.execute(built_request).await {
            Ok(response) => response,
            Err(err) => {
                return Err(format!(
                    "Failed to execute request to database. Maybe it's offline? {err}"
                )
                .into());
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
    pub async fn get_all_guild_lua_commands(
        &self,
        guild_id: GuildId,
    ) -> Result<Vec<LuaCommandInfo>, crate::Error> {
        let lua_command_infos: Option<Vec<LuaCommandInfo>> = self
            .query(format!(
                "SELECT VALUE lua_commands FROM guild:{guild_id} WHERE lua_commands;"
            ))
            .await?
            .take(0)?;

        Ok(lua_command_infos.unwrap_or_default())
    }

    pub async fn add_guild_lua_command(
        &self,
        lua_command_info: &LuaCommandInfo,
        guild_id: GuildId,
    ) -> Result<(), crate::Error> {
        let lua_command_info_string = serde_json::to_string(&lua_command_info)?;

        //TODO: Perhaps check for error.
        let _responses = self
            .query(format!(
                "UPDATE guild:{guild_id} SET lua_commands += {lua_command_info_string};"
            ))
            .await?;

        Ok(())
    }

    pub async fn remove_guild_lua_command(
        &self,
        guild_id: GuildId,
        index: usize,
    ) -> Result<(), Error> {
        let err = self
            .query(format!(
            "LET $commands = (SELECT VALUE lua_commands FROM guild:{guild_id} WHERE lua_commands);

            IF array::len($commands) == 0 {{
                THROW \"Index isn't valid.\";
            }} ELSE IF array::len($commands[0]) <= {index} {{
                THROW \"Index isn't valid.\";
            }} ELSE {{
                RETURN (UPDATE guild:{guild_id} SET lua_commands = array::remove(lua_commands, {index}));
            }};"
            ))
            .await?
            .take_err(1);

        if let Some(err) = err {
            return Err(err);
        }

        Ok(())
    }

    pub async fn get_guild_cotd_role(
        &self,
        guild_id: GuildId,
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
        guild_id: GuildId,
    ) -> Result<(), crate::Error> {
        let cotd_role_data_string = serde_json::to_string(&cotd_role_data)?;

        //TODO: Perhaps check for error.
        let _cotd_role_data = self
            .query(format!(
                "UPDATE guild:{guild_id} SET cotd_role = {cotd_role_data_string};"
            ))
            .await?;

        Ok(())
    }

    pub async fn mark_cotd_role_updated(
        &self,
        guild_id: GuildId,
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
                THROW \"Index isn't valid.\";
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

    pub async fn get_guild_message(
        &self,
        guild_id: &GuildId,
        message_id: &MessageId,
    ) -> Result<Option<StoredMessageData>, Error> {
        let res = self
            .query(format!(
                "SELECT VALUE messages.{message_id} from guild:{guild_id};"
            ))
            .await?
            .take(0)?;

        Ok(res)
    }

    pub async fn get_guild_messages(
        &self,
        guild_id: &GuildId,
    ) -> Result<Option<HashMap<MessageId, StoredMessageData>>, Error> {
        let res = self
            .query(format!("SELECT VALUE messages from guild:{guild_id};"))
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

    pub async fn set_guild_message(
        &self,
        guild_id: &GuildId,
        message_id: &MessageId,
        message_data: Option<&StoredMessageData>,
    ) -> Result<(), Error> {
        let message_data_json = serde_json::to_string(&message_data).unwrap();
        self.query(format!(
            "UPDATE guild:{guild_id} SET messages.{message_id} = {message_data_json}"
        ))
        .await?;
        Ok(())
    }

    pub async fn get_user_timezone(&self, user_id: &UserId) -> Result<Option<String>, Error> {
        let timezone: Option<String> = self
            .query(format!("SELECT VALUE timezone FROM user:{user_id};"))
            .await?
            .take(0)?;
        Ok(timezone)
    }

    pub async fn set_user_timezone(
        &self,
        user_id: &UserId,
        timezone: Option<String>,
    ) -> Result<(), Error> {
        let timezone_string = serde_json::to_string(&timezone).unwrap();
        let err = self
            .query(format!(
                "UPDATE user:{user_id} SET timezone = {timezone_string};"
            ))
            .await?
            .take_err(0);
        if let Some(err) = err {
            return Err(err);
        }
        Ok(())
    }

    pub async fn get_user_time_format(
        &self,
        user_id: &UserId,
    ) -> Result<Option<TimeFormat>, Error> {
        let time_format: Option<TimeFormat> = self
            .query(format!("SELECT VALUE time_format FROM user:{user_id};"))
            .await?
            .take(0)?;
        Ok(time_format)
    }

    pub async fn set_user_time_format(
        &self,
        user_id: &UserId,
        time_format: Option<TimeFormat>,
    ) -> Result<(), Error> {
        let time_format_string = serde_json::to_string(&time_format).unwrap();
        let err = self
            .query(format!(
                "UPDATE user:{user_id} SET time_format = {time_format_string};"
            ))
            .await?
            .take_err(0);
        if let Some(err) = err {
            return Err(err);
        }
        Ok(())
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

    pub async fn get_next_reminder_time(&self) -> Result<Option<u64>, Error> {
        let reminders = self
            .query(format!(
                "SELECT VALUE finish_time FROM reminder ORDER BY finish_time LIMIT 1;"
            ))
            .await?
            .take(0)?;

        Ok(reminders)
    }
}
