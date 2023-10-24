use super::Commands;

mod message;

pub fn get_commands() -> Commands {
    return vec![message::message()];
}
