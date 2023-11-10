use super::Commands;

mod cotd;
mod echo;
mod icon;
mod joinorder;
mod ping;
mod read;
mod rng;
mod sleepcalc;
mod write;

pub fn get_commands() -> Commands {
    return vec![
        ping::ping(),
        rng::rng(),
        read::read(),
        write::write(),
        icon::icon(),
        echo::echo(),
        sleepcalc::sleepcalc(),
        joinorder::joinorder(),
        cotd::cotd(),
    ];
}
