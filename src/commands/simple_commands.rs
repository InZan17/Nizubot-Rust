use super::Commands;

mod cotd;
mod echo;
mod eval;
mod icon;
mod join_order;
mod ping;
mod rng;
mod sleep_calc;

pub fn get_commands() -> Commands {
    return vec![
        ping::ping(),
        rng::rng(),
        icon::icon(),
        echo::echo(),
        sleep_calc::sleep_calc(),
        join_order::join_order(),
        cotd::cotd(),
        eval::eval(),
    ];
}
