use crate::{utils::IdType, Context, Error};

/// Clears all data I have on this guild/user. (Things such as reminders and other data will be reset.)
#[poise::command(slash_command, default_member_permissions = "ADMINISTRATOR")]
pub async fn clear_data(ctx: Context<'_>) -> Result<(), Error> {
    let db = &ctx.data().db;

    let id;

    if let Some(guild_id) = ctx.guild_id() {
        id = IdType::GuildId(guild_id);
    } else {
        id = IdType::UserId(ctx.author().id);
    }

    let table_id = id.into_db_table();
    let res = db
        .query(format!(
            "
        FOR $reminder IN (SELECT VALUE ->reminds->reminder FROM {table_id}) {{
            DELETE $reminder;
        }};
        DELETE {table_id};
    "
        ))
        .await?;

    if id.is_user() {
        ctx.reply("Successfully removed user data.").await?;
    } else {
        ctx.reply("Successfully removed guild data.").await?;
    }

    Ok(())
}
