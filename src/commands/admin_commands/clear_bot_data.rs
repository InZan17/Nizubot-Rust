use poise::serenity_prelude::UserId;

use crate::{utils::IdType, Context, Error};

/// Clears all bot data for this guild/user. (Things such as reminders and other data will be reset.)
#[poise::command(slash_command, default_member_permissions = "ADMINISTRATOR")]
pub async fn clear_bot_data(
    ctx: Context<'_>,
    #[description = "Are you sure you want to clear the bot data?"] confirmation: Option<bool>,
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
    let response: Option<Vec<UserId>> = db
        .query(format!(
            "
        BEGIN TRANSACTION;
        LET $uids = array::distinct(
            SELECT VALUE user_id FROM (
                SELECT VALUE ->reminds->reminder FROM {table_id}
            )
        );

        FOR $reminder IN (SELECT VALUE ->reminds->reminder FROM {table_id}) {{
            DELETE $reminder;
        }};
        DELETE {table_id};

        RETURN $uids;
        COMMIT TRANSACTION;
    "
        ))
        .await?
        .take(0)?;

    let mut affected_remind_user_ids = response.unwrap_or_default();

    affected_remind_user_ids.sort();
    affected_remind_user_ids.dedup();

    // Clear cache from all the managers.

    let remind_manager = &ctx.data().remind_manager;
    let reminders_data_read = remind_manager.reminders_data.read().await;

    for user_id in affected_remind_user_ids {
        let Some(reminders_data) = reminders_data_read.get(&user_id) else {
            continue;
        };
        reminders_data.lock().await.reminders = None;
    }

    let detector_manager = &ctx.data().detector_manager;
    let detector_manager_read = detector_manager.detectors_data.read().await;
    if let Some(detectors_data) = detector_manager_read.get(&id) {
        detectors_data.lock().await.detectors = None;
    }

    match id {
        IdType::UserId(user_id) => {
            let profile_manager = &ctx.data().profile_manager;
            let profile_manager_read = profile_manager.profiles.read().await;
            if let Some(profiles_data) = profile_manager_read.get(&user_id) {
                profiles_data.lock().await.profile = None;
            }
        }
        IdType::GuildId(guild_id) => {
            let lua_manager = &ctx.data().lua_manager;
            let lua_manager_read = lua_manager.guild_data.read().await;
            if let Some(guild_lua_data) = lua_manager_read.get(&guild_id) {
                guild_lua_data.lock().await.commands = None;
            }

            let reaction_manager = &ctx.data().reaction_manager;
            let reaction_manager_read = reaction_manager.messages_data.read().await;
            if let Some(reactions_data) = reaction_manager_read.get(&guild_id) {
                reactions_data.lock().await.messages = None;
            }
        }
    }

    if id.is_user() {
        ctx.reply("Successfully removed user data.").await?;
    } else {
        ctx.reply("Successfully removed guild data.").await?;
    }

    Ok(())
}
