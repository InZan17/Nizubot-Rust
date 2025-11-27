use poise::CreateReply;

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
    subcommand_required,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn sleep_calc(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Calculates the best time to go to sleep.
#[poise::command(slash_command)]
pub async fn sleep(
    ctx: Context<'_>,
    #[description = "At what hour do you wanna wake up?"]
    #[max = 24]
    #[min = 0]
    wake_hour: i16,
    #[description = "At what minute do you wanna wake up?"]
    #[max = 60]
    #[min = 0]
    wake_minute: i16,
    #[description = "What format is the time in?"] time_format: TimeFormat,
    #[description = "How many minutes does it take for you to fall asleep? (Default: 15)"]
    #[max = 120]
    #[min = 0]
    sleep_duration: Option<i16>,
    #[description = "How many minutes does a sleep cycle take for you? (Default: 90)"]
    #[max = 720]
    #[min = 0]
    cycle_length: Option<i16>,
    #[description = "Should the message be hidden from others?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let sleep_duration = sleep_duration.unwrap_or(15);
    let cycle_length = cycle_length.unwrap_or(90);
    let ephemeral = ephemeral.unwrap_or(false);
    let cycles = vec![
        time_string(time_after_cycle(
            wake_hour,
            wake_minute,
            time_format,
            sleep_duration,
            cycle_length,
            -0,
        )),
        time_string(time_after_cycle(
            wake_hour,
            wake_minute,
            time_format,
            sleep_duration,
            cycle_length,
            -1,
        )),
        time_string(time_after_cycle(
            wake_hour,
            wake_minute,
            time_format,
            sleep_duration,
            cycle_length,
            -2,
        )),
        time_string(time_after_cycle(
            wake_hour,
            wake_minute,
            time_format,
            sleep_duration,
            cycle_length,
            -3,
        )),
        time_string(time_after_cycle(
            wake_hour,
            wake_minute,
            time_format,
            sleep_duration,
            cycle_length,
            -4,
        )),
        time_string(time_after_cycle(
            wake_hour,
            wake_minute,
            time_format,
            sleep_duration,
            cycle_length,
            -5,
        )),
        time_string(time_after_cycle(
            wake_hour,
            wake_minute,
            time_format,
            sleep_duration,
            cycle_length,
            -6,
        )),
    ];

    ctx.send(
        CreateReply::default()
            .content(gen_wake_message(cycles))
            .ephemeral(ephemeral),
    )
    .await?;
    Ok(())
}

/// Calculates the best time to wake up.
#[poise::command(slash_command)]
pub async fn wake(
    ctx: Context<'_>,
    #[description = "At what hour do you wanna go to sleep?"]
    #[max = 24]
    #[min = 0]
    sleep_hour: i16,
    #[description = "At what minute do you wanna go to sleep?"]
    #[max = 60]
    #[min = 0]
    sleep_minute: i16,
    #[description = "What format is the time in?"] time_format: TimeFormat,
    #[description = "How many minutes does it take for you to fall asleep? (Default: 15)"]
    #[max = 120]
    #[min = 0]
    sleep_duration: Option<i16>,
    #[description = "How many minutes does a sleep cycle take for you? (Default: 90)"]
    #[max = 720]
    #[min = 0]
    cycle_length: Option<i16>,
    #[description = "Should the message be hidden from others?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let sleep_duration = sleep_duration.unwrap_or(15);
    let cycle_length = cycle_length.unwrap_or(90);
    let ephemeral = ephemeral.unwrap_or(false);
    let cycles = vec![
        time_string(time_after_cycle(
            sleep_hour,
            sleep_minute,
            time_format,
            sleep_duration,
            cycle_length,
            0,
        )),
        time_string(time_after_cycle(
            sleep_hour,
            sleep_minute,
            time_format,
            sleep_duration,
            cycle_length,
            1,
        )),
        time_string(time_after_cycle(
            sleep_hour,
            sleep_minute,
            time_format,
            sleep_duration,
            cycle_length,
            2,
        )),
        time_string(time_after_cycle(
            sleep_hour,
            sleep_minute,
            time_format,
            sleep_duration,
            cycle_length,
            3,
        )),
        time_string(time_after_cycle(
            sleep_hour,
            sleep_minute,
            time_format,
            sleep_duration,
            cycle_length,
            4,
        )),
        time_string(time_after_cycle(
            sleep_hour,
            sleep_minute,
            time_format,
            sleep_duration,
            cycle_length,
            5,
        )),
        time_string(time_after_cycle(
            sleep_hour,
            sleep_minute,
            time_format,
            sleep_duration,
            cycle_length,
            6,
        )),
    ];

    ctx.send(
        CreateReply::default()
            .content(gen_sleep_message(cycles))
            .ephemeral(ephemeral),
    )
    .await?;
    Ok(())
}

/// Info about how I calculate the times.
#[poise::command(slash_command)]
pub async fn info(
    ctx: Context<'_>,
    #[description = "Should the message be hidden from others?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = ephemeral.unwrap_or(false);
    ctx.send(CreateReply::default().content("The average human takes around 15 minutes to fall asleep. Once you are asleep you will go through sleep cycles. One sleep cycle is about 90 minutes and a good night's sleep consists of 5-6 sleep cycles. It's best to wake up at the end of a cycle to help you feel more rested and ready to start the day.\nI will calculate the best time for you to sleep/wake up by using this information.")
    .ephemeral(ephemeral)).await?;
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
    sleep_duration: i16,
    cycle_length: i16,
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
        sleep_duration
    } else if cycles < 0 {
        -sleep_duration
    } else {
        0
    };

    minute = minute + minute_offset + cycle_length * cycles;
    hour = (hour + minute / 60).rem_euclid(24);
    minute = minute.rem_euclid(60);

    //if user isn't using 24h format, convert back
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
