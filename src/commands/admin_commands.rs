use super::Commands;

//mod clear_data;
mod message;

pub fn get_commands() -> Commands {
    return vec![
        message::message(),
        //clear_data::clear_data()
    ];
}
