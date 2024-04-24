use super::Commands;

mod currency;
mod remind;
pub mod time_format;
mod timezone;

pub fn get_commands() -> Commands {
    return vec![
        remind::remind(),
        currency::currency(),
        timezone::timezone(),
        time_format::time_format(),
    ];
}
