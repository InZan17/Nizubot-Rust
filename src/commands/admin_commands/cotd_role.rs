use std::vec;

use crate::{
    managers::{
        cotd_manager::{CotdRoleData, CotdRoleDataQuery},
        db::IsConnected,
        storage_manager::DataDirectories,
    },
    Context, Error,
};
use poise::serenity_prelude::{Role, RoleId};

/// COTD role.
#[poise::command(
    slash_command,
    subcommands("create", "remove"),
    subcommand_required,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn cotdrole(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Create a role which will change color based on the COTD.
#[poise::command(slash_command)]
pub async fn create(
    ctx: Context<'_>,
    #[description = "The name of the role. <cotd> is replaced by the name of the color. (Default: <cotd>)"]
    name: Option<String>,
    #[description = "If you have an existing role you wanna change instead of creating a new one."]
    role: Option<Role>,
) -> Result<(), Error> {
    let name = name.unwrap_or("<cotd>".to_owned());

    let data = ctx.data();

    data.db.is_connected().await?;

    let guild = ctx.guild().unwrap();

    let guild_id = guild.id.as_u64().clone();

    let table_id = format!("guild:{guild_id}");

    let cotd_role_data_vec: Vec<CotdRoleDataQuery> = data
        .db
        .query(format!(
            "SELECT id, cotd_role FROM {table_id} WHERE cotd_role;"
        ))
        .await?
        .take(0)?;

    if cotd_role_data_vec.len() > 1 {
        return Err("Multiple guilds with same id??".into());
    }

    if cotd_role_data_vec.len() == 1 {
        let role_id = cotd_role_data_vec[0].cotd_role.id;
        let guild_roles = &guild.roles;
        if guild_roles.contains_key(&RoleId(role_id)) {
            ctx.send(|m| {
                m.content(format!("You already have a COTD role! <@&{role_id}>",))
                    .ephemeral(true)
            })
            .await?;
            return Ok(());
        }
    }

    let cotd_role;

    if let Some(role) = role {
        cotd_role = role
    } else {
        cotd_role = guild.create_role(ctx, |e| e.name(name.clone())).await?;
    }

    let cotd_manager = &ctx.data().cotd_manager;

    let day = cotd_manager.get_current_day();
    let role_id = cotd_role.id.as_u64().clone();

    let res = cotd_manager.update_role(ctx, cotd_role, &name).await;

    if let Err(err) = res {
        ctx.send(|m| {
            m.content(format!("Sorry, it seems like I wasn't able to create the role properly. \n\nHere's the error:\n{err}")).ephemeral(true)
        }).await?;
        return Ok(());
    }

    let cotd_role_info = CotdRoleData {
        id: role_id,
        day,
        name,
    };

    let cotd_role_data_string = serde_json::to_string(&cotd_role_info)?;

    data.db
        .query(format!(
            "UPDATE {table_id} SET cotd_role = {cotd_role_data_string};"
        ))
        .await?;

    ctx.send(|m| {
        m.content(format!("Successfully made <@&{role_id}> a COTD role.\nPlease remember to not put this role above my highest role or else I wont be able to edit it.")).ephemeral(true)
    }).await?;

    Ok(())
}

/// Stop changing the color of your COTD role.
#[poise::command(slash_command)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "If you wanna delete the role from the guild or not. (Default: False)"] delete: Option<bool>,
) -> Result<(), Error> {
    let data = ctx.data();

    data.db.is_connected().await?;

    let guild = ctx.guild().unwrap();

    let guild_id = guild.id.as_u64().clone();

    let table_id = format!("guild:{guild_id}");

    let cotd_role_data_vec: Vec<CotdRoleDataQuery> = data
        .db
        .query(format!(
            "SELECT id, cotd_role FROM {table_id} WHERE cotd_role;"
        ))
        .await?
        .take(0)?;

    if cotd_role_data_vec.len() > 1 {
        return Err("Multiple guilds with same id??".into());
    }

    if cotd_role_data_vec.len() == 0 {
        ctx.send(|m| m.content("This guild does not have a COTD role."))
            .await?;
        return Ok(());
    }
    let role_id = cotd_role_data_vec[0].cotd_role.id;

    data.db
        .query(format!("UPDATE {table_id} SET cotd_role = NONE;"))
        .await?;

    if delete.unwrap_or(false) {
        let res = ctx.guild().unwrap().delete_role(ctx, RoleId(role_id)).await;
        if let Err(err) = res {
            ctx.send(|m| {
                m.content(format!(
                    "<@&{}> is no longer a COTD role but I was unable to delete it.\n\n{}",
                    role_id,
                    err.to_string()
                ))
            })
            .await?;
        } else {
            ctx.send(|m| m.content(format!("<@&{}> has been successfully deleted.", role_id)))
                .await?;
        }
        return Ok(());
    }

    ctx.send(|m| m.content(format!("<@&{}> is no longer a COTD role.", role_id)))
        .await?;

    return Ok(());
}
