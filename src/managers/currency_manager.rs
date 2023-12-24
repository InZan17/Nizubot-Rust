use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use poise::serenity_prelude::CreateEmbed;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::Error;

use super::storage_manager::{DataHolder, StorageManager};

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
    pub storage_manager: Arc<StorageManager>,
    pub currency_info: Arc<DataHolder<CurrenciesInfo>>,
    pub list_currency_embed: RwLock<CreateEmbed>,
    token: String,
}

const SECONDS_IN_HOUR: u64 = 3600;
const SECONDS_IN_DAY: u64 = SECONDS_IN_HOUR * 24;
const SECONDS_IN_WEEK: u64 = SECONDS_IN_DAY * 7;

const API_LINK: &str = "https://openexchangerates.org/api/";

impl CurrencyManager {
    pub async fn new(storage_manager: Arc<StorageManager>, token: String) -> Self {
        let currency_info = storage_manager
            .get_data_or_default(vec!["currecy_info"], CurrenciesInfo::default())
            .await;

        let self_manager = Self {
            storage_manager,
            currency_info,
            list_currency_embed: RwLock::new(CreateEmbed::default()),
            token,
        };

        self_manager.update_embed().await;

        self_manager
    }

    pub async fn update_embed(&self) {
        let currency_info = self.currency_info.get_data().await;
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

        new_embed.field("", "", false);
        new_embed.field("More Currencies", "For a list of all supported currencies, go here: https://docs.openexchangerates.org/reference/supported-currencies", false);

        *current_embed = new_embed;
    }

    pub async fn get_embed(&self) -> CreateEmbed {
        self.list_currency_embed.read().await.clone()
    }

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

    pub async fn update_data(&self) -> Result<(), Error> {
        let currency_info = &self.currency_info;

        let currency_info_read = currency_info.get_data().await;

        if currency_info_read.rates_last_updated >= get_seconds() - SECONDS_IN_HOUR {
            if currency_info_read.names_last_updated >= get_seconds() - SECONDS_IN_WEEK {
                return Ok(());
            }
        }

        drop(currency_info_read);

        let mut currency_info_mut = currency_info.get_data_mut().await;

        if currency_info_mut.rates_last_updated < get_seconds() - SECONDS_IN_HOUR {
            let new_rates = self.get_rates().await;
            match new_rates {
                Ok(new_rates) => {
                    currency_info_mut.rates = new_rates;
                    currency_info_mut.rates_last_updated = get_seconds();
                    currency_info.request_file_write().await;
                }
                Err(err) => return Err(err),
            }
        }

        if currency_info_mut.names_last_updated < get_seconds() - SECONDS_IN_WEEK {
            let new_names = self.get_names().await;
            match new_names {
                Ok(new_names) => {
                    currency_info_mut.names = new_names;
                    currency_info_mut.names_last_updated = get_seconds();
                    currency_info.request_file_write().await;

                    drop(currency_info_mut); //we drop it for the update_embed method
                    self.update_embed().await;
                }
                Err(err) => return Err(err),
            }
        }

        Ok(())
    }

    pub async fn convert(
        &self,
        amount: f64,
        from: &String,
        to: &String,
    ) -> Result<(f64, u64), Error> {
        let storage_manager = &self.storage_manager;

        let currencies_info = storage_manager
            .get_data_or_default(vec!["currecy_info"], CurrenciesInfo::default())
            .await;

        self.update_data().await?;

        let currencies_info_read = currencies_info.get_data().await;

        let Some(from_rate) = currencies_info_read
            .rates
            .rates
            .get(&from.to_ascii_uppercase())
        else {
            return Err(Error::from(format!("{from} does not exist.")));
        };

        let Some(to_rate) = currencies_info_read
            .rates
            .rates
            .get(&to.to_ascii_uppercase())
        else {
            return Err(Error::from(format!("{to} does not exist.")));
        };

        let converted = amount / from_rate * to_rate;

        return Ok((converted, currencies_info_read.rates.timestamp));
    }

    pub async fn get_full_name(&self, currency: &String) -> Option<String> {
        let storage_manager = &self.storage_manager;

        let currencies_info = storage_manager
            .get_data_or_default(vec!["currecy_info"], CurrenciesInfo::default())
            .await;

        let currencies_info_read = currencies_info.get_data().await;

        currencies_info_read
            .names
            .get(&currency.to_ascii_uppercase())
            .cloned()
    }
}

fn get_seconds() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards. Oopsie.");

    since_the_epoch.as_secs()
}
