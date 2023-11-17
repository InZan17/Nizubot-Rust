use super::Commands;

//mod clear_data;
mod message;
pub mod cotd_role;

pub fn get_commands() -> Commands {
    return vec![
        message::message(),
        cotd_role::cotdrole()
        //clear_data::clear_data()
    ];
}
