use crate::{Context, Error};

/// Clears all data I have on this guild/user. (Things such as reminders and other data will be reset.)
#[poise::command(slash_command, default_member_permissions = "ADMINISTRATOR")]
pub async fn clear_data(ctx: Context<'_>) -> Result<(), Error> {
    if let Some(guild_id) = ctx.guild_id() {
        todo!();
        ctx.reply("Successfully removed guild data.").await?;
    } else {
        todo!();
        ctx.reply("Successfully removed user data.").await?;
    }
    Ok(())
}
