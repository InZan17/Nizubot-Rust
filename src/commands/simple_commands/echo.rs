use poise::{serenity_prelude::CreateAllowedMentions, CreateReply};

use crate::{Context, Error};

/// I will say what you want!
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn echo(
    ctx: Context<'_>,
    #[max_length = 2000]
    #[description = "What should I say?"]
    content: String,
    #[description = "Should the message be hidden from others? (Default: False)"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = ephemeral.unwrap_or(false);
    ctx.send(
        CreateReply::default()
            .content(content)
            // TODO: make sure this works
            .allowed_mentions(CreateAllowedMentions::new())
            .ephemeral(ephemeral),
    )
    .await?;
    return Ok(());
}
