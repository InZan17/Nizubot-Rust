use crate::Data;
pub type Commands =
    Vec<poise::Command<Data, Box<(dyn std::error::Error + std::marker::Send + Sync + 'static)>>>;

mod admin_commands;
mod simple_commands;
mod utility_commands;
mod fun_commands;

pub fn get_commands() -> Commands {
    let mut commands_groups: Vec<Commands> = vec![
        simple_commands::get_commands(),
        admin_commands::get_commands(),
        utility_commands::get_commands(),
        fun_commands::get_commands(),
    ];

    let mut all_commands = vec![];

    for commands in commands_groups.iter_mut() {
        all_commands.append(commands)
    }

    return all_commands;
}
