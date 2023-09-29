use super::Commands;

mod ping;

pub fn get_commands() -> Commands {
    return vec![
        ping::ping()
    ]
}