use crate::{Context, Error};

/// I will say what you want!
#[poise::command(slash_command)]
pub async fn echo(
    ctx: Context<'_>,
    #[description = "What should I say?"] message_id: String,
) -> Result<(), Error> {
    ctx.send(|m| m.content(message_id).allowed_mentions(|a| a.empty_parse()))
        .await?;
    return Ok(());
}
