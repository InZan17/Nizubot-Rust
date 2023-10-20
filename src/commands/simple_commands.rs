use super::Commands;

mod icon;
mod ping;
mod read;
mod rng;
mod write;

pub fn get_commands() -> Commands {
    return vec![
        ping::ping(),
        rng::rng(),
        read::read(),
        write::write(),
        icon::icon(),
    ];
}
