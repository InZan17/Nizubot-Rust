use crate::{Context, Error};

pub mod command;
use command::command;

/// Create your own commands! (Requires something idk)
#[poise::command(
    slash_command,
    install_context = "Guild",
    interaction_context = "Guild",
    subcommands("command"),
    subcommand_required,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn lua(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}
