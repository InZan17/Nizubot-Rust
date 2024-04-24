use super::Commands;

mod currency;
mod remind;
mod timezone;

pub fn get_commands() -> Commands {
    return vec![remind::remind(), currency::currency(), timezone::timezone()];
}
