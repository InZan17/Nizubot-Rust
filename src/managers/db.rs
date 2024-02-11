use std::{future::IntoFuture, time::Duration};

use reqwest::{Client, RequestBuilder};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;

use crate::{
    tokens::{self, SurrealDbSignInInfo},
    Error,
};

pub struct SurrealClient {
    client: Client,
    sign_in_info: SurrealDbSignInInfo,
}

trait OptionOrVec {
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

pub struct Responses(Vec<DbResponse>);

impl Responses {
    pub fn take<T: OptionOrVec + DeserializeOwned>(&self, index: usize) -> Result<T, Error> {
        //TODO check if status is OK
        if index >= self.0.len() {
            return Err("Index too high.".into());
        }

        let response = &self.0[index];

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

        //TODO: all errors gets sent to the discord slash command. Make sure all errors are safe to show
        let built_request = builder.build()?;
        let result = self.client.execute(built_request).await?;

        if !result.status().is_success() {
            println!("Failed query: {}", query);
            return Err("error occured not succes :(".into()); //TODO: fix
        }

        let db_responses = result.json::<Vec<DbResponse>>().await?;

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
