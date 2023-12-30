use std::fs;

use serde::Deserialize;

pub const DISCORD_TOKEN_PATH: &str = "./token";
pub const SURREALDB_SIGNIN_INFO_PATH: &str = "./surrealdb_signin.json";
pub const OPENEXCHANGERATES_KEY_PATH: &str = "./openExchangeRatesApiKey";

pub struct Tokens {
    pub openexchangerates_token: Option<String>,
}

pub fn get_other_tokens() -> Tokens {
    let openexchangerates_token = fs::read_to_string(OPENEXCHANGERATES_KEY_PATH);
    if let Err(err) = &openexchangerates_token {
        println!("Couldn't read file 'openExchangeRatesApiKey'. The /currency command will not work.\n{}", err.to_string())
    }

    Tokens {
        openexchangerates_token: openexchangerates_token.ok(),
    }
}

pub fn get_discord_token() -> String {
    fs::read_to_string(DISCORD_TOKEN_PATH).expect("Cannot find token file.")
}

#[derive(Debug, Deserialize)]
pub struct SurrealDbSignInInfo {
    pub address: String,
    pub namespace: String,
    pub database: String,
    pub username: String,
    pub password: String,
}

pub fn get_surreal_signin_info() -> SurrealDbSignInInfo {
    let json_data =
        fs::read_to_string(SURREALDB_SIGNIN_INFO_PATH).expect("Cannot find surrealdb_signin.json.");
    serde_json::from_str(&json_data).expect("Cannot deserialize surrealdb_signin.json.")
}
