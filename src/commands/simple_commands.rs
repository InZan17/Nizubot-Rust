use super::Commands;

mod breakread;
mod ping;
mod read;
mod rng;
mod write;

pub fn get_commands() -> Commands {
    return vec![
        ping::ping(),
        rng::rng(),
        read::read(),
        breakread::breakread(),
        write::write(),
    ];
}
