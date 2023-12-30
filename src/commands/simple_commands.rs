use super::Commands;

mod cotd;
mod echo;
mod eval;
mod icon;
mod joinorder;
mod ping;
mod read_write;
mod rng;
mod sleepcalc;

pub fn get_commands() -> Commands {
    return vec![
        ping::ping(),
        rng::rng(),
        read_write::read(),
        read_write::write(),
        icon::icon(),
        echo::echo(),
        sleepcalc::sleepcalc(),
        joinorder::joinorder(),
        cotd::cotd(),
        eval::eval(),
    ];
}
