use std::fs;

pub const DISCORD_TOKEN_PATH: &str = "./token";
pub const OPENEXCHANGERATES_KEY_PATH: &str = "./openExchangeRatesApiKey";

pub struct Tokens {
    openexchangerates_token: Option<String>
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
    fs::read_to_string(DISCORD_TOKEN_PATH).expect("Cannot read token file.")
}
