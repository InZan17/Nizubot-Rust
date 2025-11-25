use poise::CreateReply;

use crate::{Context, Error};

#[poise::command(
    slash_command,
    install_context = "Guild",
    interaction_context = "Guild",
    subcommands("restart"),
    subcommand_required,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn instance(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Restart the Lua instance.
#[poise::command(slash_command)]
pub async fn restart(
    ctx: Context<'_>,
    #[description = "Do you want all currently running commands to be stopped? (Default: False)"]
    force_quit: Option<bool>,
) -> Result<(), Error> {
    let force_quit = force_quit.unwrap_or(false);

    let data = ctx.data();

    let guild_id = ctx.guild_id().unwrap();

    let guild_lua_data = data.lua_manager.get_guild_lua_data(guild_id).await;

    guild_lua_data.lock().await.restart(force_quit);

    ctx.send(
        CreateReply::default()
            .content("Successfully restarted the Lua instance.")
            .ephemeral(true),
    )
    .await?;

    Ok(())
}
