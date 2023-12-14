use crate::{Context, Error};

/// Clears all data I have on this guild/user. (Things such as reminders and other data will be reset.)
#[poise::command(slash_command, default_member_permissions = "ADMINISTRATOR")]
pub async fn clear_data(ctx: Context<'_>) -> Result<(), Error> {
    if let Some(guild_id) = ctx.guild_id() {
        ctx.data()
            .storage_manager
            .delete_data(vec!["guilds", guild_id.to_string().as_str()])
            .await;
        ctx.reply("Successfully removed guild data.").await?;
    } else {
        ctx.data()
            .storage_manager
            .delete_data(vec!["users", ctx.author().id.to_string().as_str()])
            .await;
        ctx.reply("Successfully removed user data.").await?;
    }
    Ok(())
}
