use super::Commands;

mod clear_data;
pub mod cotd_role;
mod message;

pub fn get_commands() -> Commands {
    return vec![
        message::message(),
        cotd_role::cotdrole(),
        clear_data::clear_data()
    ];
}
