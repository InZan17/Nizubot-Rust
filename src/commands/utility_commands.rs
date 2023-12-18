use super::Commands;

mod currency;
mod remind;

pub fn get_commands() -> Commands {
    return vec![remind::remind(), currency::currency()];
}
