use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use poise::serenity_prelude::CreateEmbed;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::Error;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CurrencyRates {
    rates: HashMap<String, f64>,
    timestamp: u64,
    base: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CurrenciesInfo {
    rates: CurrencyRates,
    names: HashMap<String, String>,
    rates_last_updated: u64,
    names_last_updated: u64,
}

pub struct CurrencyManager {
    pub currency_info: RwLock<CurrenciesInfo>,
    pub list_currency_embed: RwLock<CreateEmbed>,
    token: String,
}

const SECONDS_IN_HOUR: u64 = 3600;
const SECONDS_IN_DAY: u64 = SECONDS_IN_HOUR * 24;
const SECONDS_IN_WEEK: u64 = SECONDS_IN_DAY * 7;

const API_LINK: &str = "https://openexchangerates.org/api/";

impl CurrencyManager {
    pub async fn new(token: String) -> Self {
        let self_manager = Self {
            currency_info: RwLock::new(CurrenciesInfo::default()),
            list_currency_embed: RwLock::new(CreateEmbed::default()),
            token,
        };

        self_manager.update_embed().await;

        self_manager
    }

    /// Updates the premade embed used for listing available currencies.
    pub async fn update_embed(&self) {
        let currency_info = self.currency_info.read().await;
        let mut current_embed = self.list_currency_embed.write().await;

        let mut new_embed = CreateEmbed::default();

        new_embed.title("Currency acronyms/abbreviations.")
            .description("A list of most currencies along with their acronyms/abbreviations. When running `/currency convert`, you will need to provide the currencies acronyms/abbreviations, not their full name.")
            .field("", "", false);

        const LIST_OF_CURRENCIES: [&str; 21] = [
            "USD", "EUR", "JPY", "GBP", "CNY", "AUD", "CAD", "SEK", "KRW", "NOK", "NZD", "MXN",
            "TWD", "BRL", "DKK", "PLN", "THB", "ILS", "CZK", "PHP", "RUB",
        ];

        for currency in LIST_OF_CURRENCIES {
            new_embed.field(
                currency_info
                    .names
                    .get(currency)
                    .cloned()
                    .unwrap_or("Unavailable".to_string()),
                currency,
                true,
            );
        }

        new_embed.field("", "", false); //used as padding
        new_embed.field("More Currencies", "For a list of all supported currencies, go here: https://docs.openexchangerates.org/reference/supported-currencies", false);

        *current_embed = new_embed;
    }

    /// Gets the premade embed used for listing available currencies.
    pub async fn get_embed(&self) -> CreateEmbed {
        self.list_currency_embed.read().await.clone()
    }

    /// Gets the value of currencies relative to the US dollar.
    ///
    /// Returns error if unable to connect to link or unable to parse result.
    pub async fn get_rates(&self) -> Result<CurrencyRates, Error> {
        let response = reqwest::get(format!(
            "{API_LINK}latest.json?show_alternative=1&app_id={}",
            self.token
        ))
        .await?;

        let status = response.status();
        if status != 200 {
            return Err(Error::from(format!(
                "Couldn't get currency rates. Status code: {status}"
            )));
        }

        let json_res = response.json::<CurrencyRates>().await?;
        Ok(json_res)
    }

    /// Gets the names of the currencies.
    ///
    /// Returns error if unable to connect to link or unable to parse result.
    pub async fn get_names(&self) -> Result<HashMap<String, String>, Error> {
        let response =
            reqwest::get(format!("{API_LINK}currencies.json?show_alternative=1")).await?;

        let status = response.status();
        if status != 200 {
            return Err(Error::from(format!(
                "Couldn't get currency names. Status code: {status}"
            )));
        }

        let json_res = response.json::<HashMap<String, String>>().await?;
        Ok(json_res)
    }

    /// Updates currency names and rates, but only if they are out of date.
    ///
    /// Returns Ok if nothing is out of date or if the update was successful.
    ///
    /// Returns error self.get_rates() or self.get_names() fails.
    pub async fn update_data(&self) -> Result<(), Error> {
        let currency_info = &self.currency_info;

        let currency_info_read = currency_info.read().await;

        let seconds = get_seconds();

        let rates_updated = currency_info_read.rates_last_updated >= seconds - SECONDS_IN_HOUR;
        let names_updated = currency_info_read.names_last_updated >= seconds - SECONDS_IN_WEEK;

        if rates_updated && names_updated {
            return Ok(());
        }

        drop(currency_info_read);

        let mut currency_info_mut = currency_info.write().await;

        // Make and check these variables again. This is because by the time we've gotten write access, it might've already been updated.
        let rates_updated = currency_info_mut.rates_last_updated >= seconds - SECONDS_IN_HOUR;
        let names_updated = currency_info_mut.names_last_updated >= seconds - SECONDS_IN_WEEK;

        if !rates_updated {
            let new_rates = self.get_rates().await?;
            currency_info_mut.rates = new_rates;
            currency_info_mut.rates_last_updated = get_seconds();
        }

        if !names_updated {
            let new_names = self.get_names().await?;
            currency_info_mut.names = new_names;
            currency_info_mut.names_last_updated = get_seconds();

            drop(currency_info_mut); //we drop it for the update_embed method
                                     //Update embed is called so that we can get the most up to date currency names in our list embed.
            self.update_embed().await;
        }

        Ok(())
    }

    /// Converts an amount of currency from one to another.
    ///
    /// Returns error if self.update_data() fails or if any currency doesn't exist in the rates list.
    ///
    /// Returns (f64, u64).
    /// The first element is the converted amount and the second is the last time it was updated.
    pub async fn convert(
        &self,
        amount: f64,
        from: &String,
        to: &String,
    ) -> Result<(f64, u64), Error> {
        self.update_data().await?;

        let currency_info = &self.currency_info;

        let currencies_info_read = currency_info.read().await;

        let rates = &currencies_info_read.rates.rates;

        let Some(from_rate) = rates.get(&from.to_ascii_uppercase()) else {
            return Err(Error::from(format!("{from} does not exist.")));
        };

        let Some(to_rate) = rates.get(&to.to_ascii_uppercase()) else {
            return Err(Error::from(format!("{to} does not exist.")));
        };

        let converted = amount / from_rate * to_rate;

        return Ok((converted, currencies_info_read.rates.timestamp));
    }

    /// Gets the full name of a currency.
    ///
    /// Returns None if currency doesn't exist in the hashmap.
    pub async fn get_full_name(&self, currency: &String) -> Option<String> {
        let currency_info = &self.currency_info;

        let currencies_info_read = currency_info.read().await;

        currencies_info_read
            .names
            .get(&currency.to_ascii_uppercase())
            .cloned()
    }
}

// TODO put this function and all the duplicates of it in a special place.
fn get_seconds() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards. Oopsie.");

    since_the_epoch.as_secs()
}
