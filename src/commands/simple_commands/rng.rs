use crate::{Context, Error};
use poise::CreateReply;
use rand::Rng;

/// I will pick a random number!
#[poise::command(slash_command)]
pub async fn rng(
    ctx: Context<'_>,
    #[description = "Smallest possible number."] min: Option<i32>,
    #[description = "Biggest possible number."] max: Option<i32>,
) -> Result<(), Error> {
    let min_unwrap = min.unwrap_or(0);
    let mut max_unwrap = max.unwrap_or(0);
    if max.is_some() || min.is_some() {
        if max_unwrap == min_unwrap {
            ctx.send(CreateReply::default().content(
                "Please make sure the difference between 'min' and 'max' are larger than 0.",
            ).ephemeral(true))
            .await?;
            return Ok(());
        }
        if max_unwrap < min_unwrap {
            ctx.send(
                CreateReply::default()
                    .content("Please make sure 'min' is less than 'max'.")
                    .ephemeral(true),
            )
            .await?;
            return Ok(());
        }
    } else {
        max_unwrap = 100;
    }

    let rng = rand::thread_rng().gen_range(min_unwrap..max_unwrap + 1);
    ctx.reply(format!("{}!", rng)).await?;
    return Ok(());
}
