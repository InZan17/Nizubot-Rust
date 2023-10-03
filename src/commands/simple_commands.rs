use super::Commands;

mod ping;
mod rng;

pub fn get_commands() -> Commands {
    return vec![
        ping::ping(),
        rng::rng()
    ]
}