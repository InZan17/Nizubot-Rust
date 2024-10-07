use poise::{serenity_prelude::CreateAllowedMentions, CreateReply};

use crate::{Context, Error};

/// I will say what you want!
#[poise::command(slash_command)]
pub async fn echo(
    ctx: Context<'_>,
    #[max_length = 2000]
    #[description = "What should I say?"]
    content: String,
) -> Result<(), Error> {
    ctx.send(
        CreateReply::default()
            .content(content)
            // TODO: make sure this works
            .allowed_mentions(CreateAllowedMentions::new()),
    )
    .await?;
    return Ok(());
}
