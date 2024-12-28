use std::time::{SystemTime, UNIX_EPOCH};

use poise::{
    serenity_prelude::{Attachment, Mentionable},
    CreateReply,
};

use crate::{Context, Error};

/// Create your own commands! (Requires something idk)
#[poise::command(
    slash_command,
    install_context = "Guild",
    interaction_context = "Guild",
    subcommands("create", "update", "delete", "refresh"),
    subcommand_required,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn lua_command(ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Creates a custom command.
#[poise::command(slash_command)]
pub async fn create(
    ctx: Context<'_>,
    command_name: String,
    lua_file: Attachment,
) -> Result<(), Error> {
    const FIFTY_KB_IN_BYTES: u32 = 50000;

    if lua_file.size > FIFTY_KB_IN_BYTES {
        ctx.send(
            CreateReply::default()
                .content("Please make sure your file is 50 KB or less in size.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    if !lua_file.filename.ends_with(".lua") && !lua_file.filename.ends_with(".luau") {
        ctx.send(
            CreateReply::default()
                .content("Please make sure your file is a lua or luau file.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let response = reqwest::get(&lua_file.url).await?;
    if !response.status().is_success() {
        return Err(Error::from(format!(
            "{} {}",
            response.status(),
            response.text().await.unwrap_or("".to_string())
        )));
    }

    let lua_code = response.text().await?;

    let data = ctx.data();

    data.lua_manager
        .register_command(
            ctx.guild_id().unwrap(),
            command_name.clone(),
            lua_code,
            lua_file.filename,
        )
        .await?;

    ctx.send(
        CreateReply::default()
            .content(format!(
                "Successfully created custom command! Try it out using /c {}",
                command_name
            ))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// Updates an existing custom command.
#[poise::command(slash_command)]
pub async fn update(
    ctx: Context<'_>,
    command_name: String,
    lua_file: Attachment,
) -> Result<(), Error> {
    const FIFTY_KB_IN_BYTES: u32 = 50000;

    if lua_file.size > FIFTY_KB_IN_BYTES {
        ctx.send(
            CreateReply::default()
                .content("Please make sure your file is 50 KB or less in size.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    if !lua_file.filename.ends_with(".lua") && !lua_file.filename.ends_with(".luau") {
        ctx.send(
            CreateReply::default()
                .content("Please make sure your file is a lua or luau file.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let response = reqwest::get(&lua_file.url).await?;
    if !response.status().is_success() {
        return Err(Error::from(format!(
            "{} {}",
            response.status(),
            response.text().await.unwrap_or("".to_string())
        )));
    }

    let lua_code = response.text().await?;

    let data = ctx.data();

    data.lua_manager
        .update_command(
            ctx.guild_id().unwrap(),
            command_name.clone(),
            lua_code,
            lua_file.filename,
        )
        .await?;

    ctx.send(
        CreateReply::default()
            .content(format!(
                "Successfully updated the custom command! Try it out using /c {}",
                command_name
            ))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// Updates an existing custom command.
#[poise::command(slash_command)]
pub async fn delete(ctx: Context<'_>, command_name: String) -> Result<(), Error> {
    let data = ctx.data();

    data.lua_manager
        .delete_command(ctx.guild_id().unwrap(), command_name.clone())
        .await?;

    ctx.send(
        CreateReply::default()
            .content("Successfully deleted the custom command.")
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// Refreshes all custom commands.
#[poise::command(slash_command)]
pub async fn refresh(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data();

    let guild_id = ctx.guild_id().unwrap();

    let command_infos = data.db.get_all_guild_lua_commands(guild_id).await?;

    data.lua_manager
        .update_guild_commands(guild_id, command_infos)
        .await?;

    ctx.send(
        CreateReply::default()
            .content("Successfully refreshed all custom commands.")
            .ephemeral(true),
    )
    .await?;

    Ok(())
}
