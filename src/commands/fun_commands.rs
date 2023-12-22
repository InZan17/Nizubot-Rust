use super::Commands;

mod genmeme;

pub fn get_commands() -> Commands {
    return vec![genmeme::genmeme()];
}
