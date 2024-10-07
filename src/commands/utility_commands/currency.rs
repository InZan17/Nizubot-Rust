use poise::{
    serenity_prelude::{CreateEmbed, CreateEmbedFooter, Timestamp},
    CreateReply,
};

use crate::{Context, Error};

/// Command about converting currencies.
#[poise::command(slash_command, subcommands("convert", "list"), subcommand_required)]
pub async fn currency(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Convert currencies from one to another.
#[poise::command(slash_command)]
pub async fn convert(
    ctx: Context<'_>,
    #[description = "The amount of currency you wanna convert."] amount: f64,
    #[description = "Currency to convert from."] from: String,
    #[description = "Currency to convert to."] to: String,
) -> Result<(), Error> {
    let currency_manager = &ctx.data().currency_manager;

    let (converted, timestamp) = currency_manager.convert(amount, &from, &to).await?;

    let from_name;

    if let Some(name) = currency_manager.get_full_name(&from).await {
        from_name = format!("({name})");
    } else {
        from_name = "".to_owned();
    }

    let to_name;

    if let Some(name) = currency_manager.get_full_name(&to).await {
        to_name = format!("({name})");
    } else {
        to_name = "".to_owned();
    }

    ctx.send(
        CreateReply::default().embed(
            CreateEmbed::new()
                .title("Currency Conversion")
                .description("Currency rates were taken from https://openexchangerates.org.")
                .footer(CreateEmbedFooter::new("Currency rates last updated"))
                .timestamp(Timestamp::from_unix_timestamp(timestamp as i64).unwrap())
                .field(
                    format!("From: {} {}", from.to_uppercase(), from_name),
                    amount.to_string(),
                    false,
                )
                .field(
                    format!("To: {} {}", to.to_uppercase(), to_name),
                    fancy_round(converted, 2).to_string(),
                    false,
                )
                .field("", "", false),
        ),
    )
    .await?;

    Ok(())
}
/// List of some currencies and their acronyms/abbreviations.
#[poise::command(slash_command)]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    let the_embed = ctx.data().currency_manager.get_embed().await;

    ctx.send(CreateReply::default().embed(the_embed)).await?;
    Ok(())
}

fn round_to_decimal(number: f64, decimals: u32) -> f64 {
    let multiply = 10.0_f64.powi(decimals as i32);
    (number * multiply).round() / multiply
}

fn fancy_round(number: f64, mut visible_decimals: u32) -> f64 {
    let number_str = number.to_string();
    let split_number: Vec<&str> = number_str.split('.').collect();
    let decimal = split_number.get(1).unwrap_or(&"");

    for (i, char) in decimal.chars().enumerate() {
        if char != '0' {
            visible_decimals += i as u32;
            break;
        }
    }

    round_to_decimal(number, visible_decimals)
}
