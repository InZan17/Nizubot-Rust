use super::Commands;

pub mod check_timezone;
mod currency;
pub mod profile;
mod remind;

pub fn get_commands() -> Commands {
    return vec![
        remind::remind(),
        currency::currency(),
        check_timezone::check_timezone(),
        profile::profile(),
    ];
}
