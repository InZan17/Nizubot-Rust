use crate::{utils::IdType, Context, Error};

/// Clears all data I have on this guild/user. (Things such as reminders and other data will be reset.)
#[poise::command(slash_command, default_member_permissions = "ADMINISTRATOR")]
pub async fn clear_data(
    ctx: Context<'_>,
    #[description = "Are you sure you want to clear the data?"] confirmation: Option<bool>,
) -> Result<(), Error> {
    let confirmation = confirmation.unwrap_or(false);

    if !confirmation {
        if ctx.guild_id().is_some() {
            ctx.reply("Are you sure you wanna clear my data about this guild? Set the `confirmation` parameter to `True` to confirm.")
                .await?;
        } else {
            ctx.reply("Are you sure you wanna clear my data about you? Set the `confirmation` parameter to `True` to confirm.")
                .await?;
        }
        return Ok(());
    }

    let db = &ctx.data().db;

    let id;

    if let Some(guild_id) = ctx.guild_id() {
        id = IdType::GuildId(guild_id);
    } else {
        id = IdType::UserId(ctx.author().id);
    }

    let table_id = id.into_db_table();
    let _res = db
        .query(format!(
            "
        FOR $reminder IN (SELECT VALUE ->reminds->reminder FROM {table_id}) {{
            DELETE $reminder;
        }};
        DELETE {table_id};
    "
        ))
        .await?;

    // TODO: log unsucessful deletion.
    if id.is_user() {
        ctx.reply("Successfully removed user data.").await?;
    } else {
        ctx.reply("Successfully removed guild data.").await?;
    }

    Ok(())
}
