use crate::{Context, Error};

#[derive(Debug, Clone, Copy, PartialEq, Eq, poise::ChoiceParameter)]
pub enum TimeFormat {
    #[name = "AM"]
    AM,
    #[name = "PM"]
    PM,
    #[name = "24h clock"]
    MT,
}

impl TimeFormat {
    fn string(&self) -> &str {
        match self {
            TimeFormat::AM => "am",
            TimeFormat::PM => "pm",
            TimeFormat::MT => "",
        }
    }
}

/// Calculates the best time to go to sleep/wake up.
#[poise::command(
    slash_command,
    subcommands("sleep", "wake", "info"),
    subcommand_required
)]
pub async fn sleepcalc(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Calculate the best time to go to sleep.
#[poise::command(slash_command)]
pub async fn sleep(
    ctx: Context<'_>,
    #[description = "Hour of the time you wanna wake up."]
    #[max = 24]
    #[min = 0]
    wake_hour: i16,
    #[description = "Miniute of the time you wanna wake up."]
    #[max = 60]
    #[min = 0]
    wake_minute: i16,
    #[description = "What format the time is in."] format: TimeFormat,
) -> Result<(), Error> {
    let cycles = vec![
        time_string(time_after_cycle(wake_hour, wake_minute, format, -0)),
        time_string(time_after_cycle(wake_hour, wake_minute, format, -1)),
        time_string(time_after_cycle(wake_hour, wake_minute, format, -2)),
        time_string(time_after_cycle(wake_hour, wake_minute, format, -3)),
        time_string(time_after_cycle(wake_hour, wake_minute, format, -4)),
        time_string(time_after_cycle(wake_hour, wake_minute, format, -5)),
        time_string(time_after_cycle(wake_hour, wake_minute, format, -6)),
    ];

    ctx.send(|m| m.content(gen_wake_message(cycles))).await?;
    Ok(())
}

/// Calculate the best time to wake up.
#[poise::command(slash_command)]
pub async fn wake(
    ctx: Context<'_>,
    #[description = "Hour of the time you wanna go to sleep."]
    #[max = 24]
    #[min = 0]
    sleep_hour: i16,
    #[description = "Miniute of the time you wanna go to sleep."]
    #[max = 60]
    #[min = 0]
    sleep_minute: i16,
    #[description = "What format the time is in."] format: TimeFormat,
) -> Result<(), Error> {
    let cycles = vec![
        time_string(time_after_cycle(sleep_hour, sleep_minute, format, 0)),
        time_string(time_after_cycle(sleep_hour, sleep_minute, format, 1)),
        time_string(time_after_cycle(sleep_hour, sleep_minute, format, 2)),
        time_string(time_after_cycle(sleep_hour, sleep_minute, format, 3)),
        time_string(time_after_cycle(sleep_hour, sleep_minute, format, 4)),
        time_string(time_after_cycle(sleep_hour, sleep_minute, format, 5)),
        time_string(time_after_cycle(sleep_hour, sleep_minute, format, 6)),
    ];

    ctx.send(|m| m.content(gen_sleep_message(cycles))).await?;
    Ok(())
}

/// Info about how I calculate the times.
#[poise::command(slash_command)]
pub async fn info(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(|m| m.content("The average human takes around 15 minutes to fall asleep. Once you are asleep you will go through sleep cycles. One sleep cycle is about 90 minutes and a good night's sleep consists of 5-6 sleep cycles. It's best to wake up at the end of a cycle to help you feel more rested and ready to start the day.\nI will calculate the best time for you to sleep/wake up by using this information.")).await?;
    return Ok(());
}

fn gen_sleep_message(cycles: Vec<String>) -> String {
    return format!("If you wanna go to sleep at `{}`, then I recommend you to wake up at `{}` or `{}`.\n\nIf you need to you can also wake up at the following times:\n`{}`\n`{}`\n`{}`\n`{}`", cycles[0], cycles[6], cycles[5], cycles[4], cycles[3], cycles[2], cycles[1]);
}

fn gen_wake_message(cycles: Vec<String>) -> String {
    return format!("If you wanna wake up at `{}`, then I recommend you go to sleep at `{}` or `{}`.\n\nIf you need to you can also go to sleep at the following times:\n`{}`\n`{}`\n`{}`\n`{}`", cycles[0], cycles[6], cycles[5], cycles[4], cycles[3], cycles[2], cycles[1]);
}

fn time_string((hour, minute, format): (i16, i16, TimeFormat)) -> String {
    let mut final_string = hour.to_string();
    if minute < 10 {
        final_string = format!("{}:0{}", final_string, minute);
    } else {
        final_string = format!("{}:{}", final_string, minute);
    }

    return format!("{}{}", final_string, format.string());
}

fn time_after_cycle(
    mut hour: i16,
    mut minute: i16,
    mut format: TimeFormat,
    cycles: i16,
) -> (i16, i16, TimeFormat) {
    //convert it to 24h format
    if format != TimeFormat::MT {
        if hour > 12 {
            if format == TimeFormat::AM {
                format = TimeFormat::PM;
            } else {
                format = TimeFormat::AM;
            }
        }
        hour = hour.rem_euclid(12);
        if format == TimeFormat::PM {
            hour = hour + 12;
        }
    }

    let minute_offset = if cycles > 0 {
        15
    } else if cycles < 0 {
        -15
    } else {
        0
    };

    minute = minute + minute_offset + 30 * cycles;
    hour = (hour + cycles + minute / 60).rem_euclid(24);
    minute = minute.rem_euclid(60);
    println!("{hour}, {minute}");

    //if user isnt using 24h format, convert back
    if format != TimeFormat::MT {
        if hour >= 12 {
            format = TimeFormat::PM
        } else {
            format = TimeFormat::AM
        }

        if hour > 12 {
            hour = hour - 12
        } else if hour == 0 {
            hour = 12
        }
    }

    return (hour, minute, format);
}
