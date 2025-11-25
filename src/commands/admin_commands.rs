use super::Commands;

mod clear_bot_data;
pub mod cotd_role;
mod detect_message;
mod log;
mod lua;
mod manage_reminders;
mod message;
mod reaction_role;

pub fn get_commands() -> Commands {
    return vec![
        message::message(),
        cotd_role::cotd_role(),
        lua::lua(),
        clear_bot_data::clear_bot_data(),
        log::log(),
        detect_message::detect_message(),
        reaction_role::reaction_role(),
        manage_reminders::manage_reminders(),
    ];
}
