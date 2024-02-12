use poise::serenity_prelude::GuildId;
use reqwest::{Client, RequestBuilder};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;

use crate::{
    tokens::{self, SurrealDbSignInInfo},
    Error,
};

use super::cotd_manager::{CotdRoleData, CotdRoleDataQuery};

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

pub async fn new_db() -> SurrealClient {
    let surreal_login_info = tokens::get_surreal_signin_info();

    SurrealClient::new(surreal_login_info)
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

impl SurrealClient {
    pub async fn get_guild_cotd_role(
        &self,
        guild_id: &GuildId,
    ) -> Result<Option<CotdRoleDataQuery>, crate::Error> {
        //TODO: remove ID from select and test it.
        let cotd_role_data: Option<CotdRoleDataQuery> = self
            .query(format!(
                "SELECT id, cotd_role FROM guild:{guild_id} WHERE cotd_role;"
            ))
            .await?
            .take(0)?;

        Ok(cotd_role_data)
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
}
