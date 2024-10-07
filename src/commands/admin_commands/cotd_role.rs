use std::vec;

use crate::{managers::cotd_manager::CotdRoleData, Context, Error};
use poise::{
    serenity_prelude::{CreateAllowedMentions, EditRole, Role},
    CreateReply,
};

/// COTD role.
#[poise::command(
    slash_command,
    subcommands("create", "remove"),
    subcommand_required,
    guild_only,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn cotdrole(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Create a role which will change color based on the COTD.
#[poise::command(slash_command, required_bot_permissions = "MANAGE_ROLES")]
pub async fn create(
    ctx: Context<'_>,
    #[max_length = 100]
    #[description = "The name of the role. <cotd> is replaced by the name of the color. (Default: <cotd>)"]
    name: Option<String>,
    #[description = "If you have an existing role you wanna change instead of creating a new one."]
    role: Option<Role>,
) -> Result<(), Error> {
    let name = name.unwrap_or("<cotd>".to_owned());

    let data = ctx.data();

    let roles;
    let guild_id;

    {
        let guild = ctx.guild().unwrap();
        guild_id = guild.id;
        roles = guild.roles.clone();
    }

    let cotd_role_data = data.db.get_guild_cotd_role(guild_id).await?;

    if let Some(cotd_role_data) = cotd_role_data {
        let role_id = cotd_role_data.cotd_role.id;
        if roles.contains_key(&role_id) {
            ctx.send(
                CreateReply::default()
                    .content(format!("You already have a COTD role! <@&{role_id}>",))
                    .ephemeral(true),
            )
            .await?;
            return Ok(());
        }
    }

    let cotd_role;

    if let Some(role) = role {
        cotd_role = role
    } else {
        cotd_role = guild_id
            .create_role(ctx, EditRole::new().name(name.clone()).position(0))
            .await?;
    }

    let cotd_manager = &ctx.data().cotd_manager;

    let day = cotd_manager.get_current_day();
    let role_id = cotd_role.id.clone();

    let current_color = match cotd_manager.get_current_color().await {
        Ok(ok) => ok,
        Err(err) => return Err(err.to_string().into()),
    };

    let res = cotd_manager
        .update_role(ctx, guild_id, cotd_role.id, &name, &current_color)
        .await;

    if let Err(err) = res {
        ctx.send(CreateReply::default().content(format!("Sorry, it seems like I wasn't able to create the role properly. \n\nHere's the error:\n{}", err.to_string())).ephemeral(true)
        ).await?;
        return Ok(());
    }

    let cotd_role_info = CotdRoleData {
        id: role_id,
        day,
        name,
    };

    data.db
        .update_guild_cotd_role(&Some(cotd_role_info), guild_id)
        .await?;

    ctx.send(CreateReply::default().content(format!("Successfully made <@&{role_id}> a COTD role.\nPlease remember to not put this role above my highest role or else I wont be able to edit it."))
        .allowed_mentions(CreateAllowedMentions::new())
    ).await?;

    Ok(())
}

/// Stop changing the color of your COTD role.
#[poise::command(slash_command)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "If you wanna delete the role from the guild or not. (Default: False)"] delete: Option<bool>,
) -> Result<(), Error> {
    let data = ctx.data();

    let guild_id;
    {
        let guild = ctx.guild().unwrap();
        guild_id = guild.id;
    }

    let cotd_role_data = data.db.get_guild_cotd_role(guild_id).await?;

    let Some(cotd_role_data) = cotd_role_data else {
        ctx.send(
            CreateReply::default()
                .content(format!("This guild does not have a COTD role.",))
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };
    let role_id = cotd_role_data.cotd_role.id;

    data.db.update_guild_cotd_role(&None, guild_id).await?;

    if delete.unwrap_or(false) {
        let res = guild_id.delete_role(ctx, role_id).await;
        if let Err(err) = res {
            ctx.send(
                CreateReply::default()
                    .content(format!(
                        "<@&{}> is no longer a COTD role but I was unable to delete it.\n\n{}",
                        role_id,
                        err.to_string()
                    ))
                    // TODO: Make sure this does not ping anything.
                    .allowed_mentions(CreateAllowedMentions::new()),
            )
            .await?;
        } else {
            ctx.send(
                CreateReply::default()
                    .content(format!("<@&{}> has been successfully deleted.", role_id))
                    // TODO: Make sure this does not ping anything.
                    .allowed_mentions(CreateAllowedMentions::new()),
            )
            .await?;
        }
        return Ok(());
    }

    ctx.send(
        CreateReply::default()
            .content(format!("<@&{}> is no longer a COTD role.", role_id))
            // TODO: Make sure this does not ping anything.
            .allowed_mentions(CreateAllowedMentions::new()),
    )
    .await?;

    return Ok(());
}
