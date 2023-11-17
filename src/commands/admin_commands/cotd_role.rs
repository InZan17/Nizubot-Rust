use std::vec;

use poise::serenity_prelude::{AttachmentType, CreateEmbed, Embed, Message, MessageType, Role, RoleId};
use serde::{Serialize, Deserialize};

use crate::{Context, Error};

#[derive(Serialize, Deserialize)]
pub struct CotdRoleInfo {
    name: String,
    day: u64,
    id: u64
}

/// COTD role.
#[poise::command(
    slash_command,
    subcommands("create", "remove"),
    subcommand_required,
    default_member_permissions = "ADMINISTRATOR",

)]
pub async fn cotdrole(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Create a role which will change color based on the COTD.
#[poise::command(slash_command)]
pub async fn create(
    ctx: Context<'_>,
    #[description = "The name of the role. <cotd> is replaced by the name of the color. (Default: <cotd>)"] name: Option<String>,
    #[description = "If you have an existing role you wanna change instead of creating a new one."] role: Option<Role>,
) -> Result<(), Error> {
    let name = name.unwrap_or("<cotd>".to_owned());

    let cotd_roles_data = ctx.data()
        .storage_manager
        .get_data_or_default::<Vec<u64>>(vec!["cotdRoles"], vec![])
        .await;
    
    let cotd_roles_read = cotd_roles_data.get_data().await;

    let guild = ctx.guild().unwrap();

    let guild_id = guild.id.as_u64().clone();

    if cotd_roles_read.contains(&guild_id) {
        let guild_cotd_role_data = ctx.data()
            .storage_manager
            .get_data::<CotdRoleInfo>(vec!["guilds", &guild_id.to_string(), "cotd_role"])
            .await;

        if let Some(guild_cotd_role_data) = guild_cotd_role_data {
            let read_data = guild_cotd_role_data.get_data().await;
            let guild_roles = &guild.roles;
            if guild_roles.contains_key(&RoleId(read_data.id)) {
                ctx.send(|m| {
                    m.content(format!("You already have a COTD role! <@&{}>", read_data.id)).ephemeral(true)
                }).await?;
                return Ok(())
            }
        }
    }

    let cotd_role;

    if let Some(role) = role {
        cotd_role = role
    } else {
        cotd_role = guild.create_role(ctx, |e| {
            e.name(name.clone())
        }).await?;
    }

    let cotd_manager = &ctx.data().cotd_manager;

    let day = cotd_manager.get_current_day();
    let role_id = cotd_role.id.as_u64().clone();

    let res = cotd_manager.update_role(ctx, cotd_role, &name).await;

    if let Err(err) = res {
        ctx.send(|m| {
            m.content(format!("Sorry, it seems like I wasn't able to create the role properly. \n\nHere's the error:\n{err}")).ephemeral(true)
        }).await?;
        return Ok(())
    }

    drop(cotd_roles_read);

    cotd_roles_data.get_data_mut().await.push(guild_id);
    cotd_roles_data.request_file_write().await;

    let cotd_role_info = CotdRoleInfo{
        id: role_id,
        day,
        name
    };

    let guild_cotd_role_data = ctx.data()
        .storage_manager
        .get_data_or_default::<CotdRoleInfo>(vec!["guilds", &guild_id.to_string(), "cotd_role"], CotdRoleInfo { name: "".to_owned(), day: 0, id: 0 })
        .await;

    *(guild_cotd_role_data.get_data_mut().await) = cotd_role_info;
    guild_cotd_role_data.request_file_write().await;


    /*

    local role, err = ia.guild:createRole(name)

    if not role then
        local code = funs.parseDiaError(err)
        if code == "30005" then
            return ia:reply("It seems like this guild has reached the max amount of roles. Try deleting some of the roles.", true)
        end
        return ia:reply("Sorry, it seems like I wasn't able to create the role. \n\nHere's the error:\n"..err, true)
    end

    local success, updateErr = _G.cotd.updateRole(role, name)

    if not success then
        return ia:reply("Sorry, it seems like I wasn't able to create the role properly. \n\nHere's the error:\n"..updateErr, true)
    end

    cotdRolesRead[ia.guild.id] = {
        id = role.id,
        name = name
    }
    cotdRolesData:write(cotdRolesRead)

    ctx.send(|m| {
        m.content(format!("Sorry, it seems like I wasn't able to create the role properly. \n\nHere's the error:\n{err}")).ephemeral(true)
    }).await?;

    ia:reply("Successfully made <@&"..role.id.."> a COTD role.\nPlease remember to not put this role above my highest role or else I wont be able to edit it.", true)
    */

    ctx.send(|m| {
        m.content(format!("Successfully made <@&{role_id}> a COTD role.\nPlease remember to not put this role above my highest role or else I wont be able to edit it.")).ephemeral(true)
    }).await?;

    Ok(())
}

/// Stop changing the color of your COTD role.
#[poise::command(slash_command)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "If you wanna delete the role from the guild or not. (Default: False)"] content: Option<bool>,
) -> Result<(), Error> {
    return Ok(());
}