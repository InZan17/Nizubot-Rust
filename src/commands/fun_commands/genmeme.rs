use std::time::{SystemTime, UNIX_EPOCH};

use poise::serenity_prelude::User;

use crate::{Context, Error};

/// I will generate a meme.
#[poise::command(slash_command, subcommands("brick"))]
pub async fn genmeme(ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Generate a gif of some user throwing a brick.
#[poise::command(slash_command)]
pub async fn brick(
    ctx: Context<'_>,
    #[description = "The user to throw the brick."] user: Option<User>,
) -> Result<(), Error> {

    let user = user.unwrap_or(ctx.author().clone());

    let avatar_url = user.avatar_url().unwrap_or(user.default_avatar_url());

    Ok(())
}