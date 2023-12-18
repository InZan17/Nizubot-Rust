use super::Commands;

mod clear_data;
pub mod cotd_role;
mod detect_message;
mod message;
mod reaction_role;

pub fn get_commands() -> Commands {
    return vec![
        message::message(),
        cotd_role::cotdrole(),
        clear_data::clear_data(),
        detect_message::detect_message(),
        reaction_role::reaction_role(),
    ];
}
