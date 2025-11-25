use std::{collections::HashMap, sync::Arc, time::Duration};

use poise::serenity_prelude::UserId;
use tokio::time::sleep;

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

    let guild_lua_data_for_lock;
    let mut lua_lock;

    if let Some(guild_id) = ctx.guild_id() {
        id = IdType::GuildId(guild_id);

        // We have to handle the lua manager early here. This is because lua manager is special.
        // When you run a normal command to change data, and at the same time someone calls clear_bot_data,
        // it doesn't really matter if there was any race conditions that made it write the data anyway.
        // But if someone runs a lua command that changes data_store data at the same time someone calls clear_bot_data,
        // now you should expect the data to not have been written, because the command also gets deleted.
        // But that wont always be the case in certain conditions, so we have to hold a lock to the lua data
        // during the db query to make sure it doesn't write any data.

        let lua_manager = &ctx.data().lua_manager;
        let lua_manager_read = lua_manager.guild_data.read().await;

        if let Some(guild_lua_data) = lua_manager_read.get(&guild_id).cloned() {
            guild_lua_data_for_lock = Some(guild_lua_data);
            let mut lock = guild_lua_data_for_lock.as_ref().unwrap().lock().await;
            lock.restart(true);
            lock.commands = None;
            lua_lock = Some(lock)
        } else {
            guild_lua_data_for_lock = None;
            lua_lock = None;
        }

        let data_stores_read = lua_manager.data_stores.read().await;
        if let Some(data_stores) = data_stores_read.get(&guild_id) {
            let mut lock = data_stores.lock().await;
            let raw_map = lock.get_raw_map();

            // Remove data store whenever it has no more references, and keep doing it until it's all gone.
            // That means Lua has been properly dropped.
            while !raw_map.is_empty() {
                sleep(Duration::from_millis(10)).await;
                raw_map.retain(|_, v| Arc::strong_count(&v.0) > 1);
            }
        }
    } else {
        id = IdType::UserId(ctx.author().id);
        guild_lua_data_for_lock = None;
        lua_lock = None;
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

    if let Some(lock) = lua_lock.as_mut() {
        // Assign the commands to be Some so the function doesn't do a request to get the lua commands.
        // The reason we don't do this for the other managers is because there may be a race condition
        // when clearing the data and adding something at the same time.
        // This doesn't happen for the lua manager because we hold the lock throughout the query.
        lock.commands = Some(HashMap::new());
    };

    drop(lua_lock);
    drop(guild_lua_data_for_lock);

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
            let reaction_manager = &ctx.data().reaction_manager;
            let reaction_manager_read = reaction_manager.messages_data.read().await;
            if let Some(reactions_data) = reaction_manager_read.get(&guild_id) {
                reactions_data.lock().await.messages = None;
            }

            // The commands were already cleared earlier.
            // Now we just update the guild commands os that the commands get removed.
            let lua_manager = &ctx.data().lua_manager;
            let guild_lua_data = lua_manager.get_guild_lua_data(guild_id).await;
            guild_lua_data
                .lock()
                .await
                .update_guild_commands(&ctx.data().db, ctx.http())
                .await?;
        }
    }

    if id.is_user() {
        ctx.reply("Successfully removed user data.").await?;
    } else {
        ctx.reply("Successfully removed guild data.").await?;
    }

    Ok(())
}
