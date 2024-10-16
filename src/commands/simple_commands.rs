use super::Commands;

mod cotd;
mod echo;
mod eval;
mod icon;
mod joinorder;
mod ping;
mod rng;
mod sleepcalc;

pub fn get_commands() -> Commands {
    return vec![
        ping::ping(),
        rng::rng(),
        icon::icon(),
        echo::echo(),
        sleepcalc::sleepcalc(),
        joinorder::joinorder(),
        cotd::cotd(),
        eval::eval(),
    ];
}
