use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use poise::{
    serenity_prelude::{self, Attachment, CreateAttachment},
    CreateReply,
};

async fn autocomplete_command_name(
    ctx: Context<'_>,
    partial: &str,
) -> Vec<poise::serenity_prelude::AutocompleteChoice> {
    let Some(guild_id) = ctx.guild_id() else {
        return vec![];
    };

    let guild_lua_data = ctx.data().lua_manager.get_guild_lua_data(guild_id).await;

    let mut commands_lock = guild_lua_data.lock().await;

    let Ok(commands) = commands_lock.get_commands(&ctx.data().db).await else {
        return vec![];
    };

    let matcher = SkimMatcherV2::default().ignore_case();

    let mut keys = commands
        .keys()
        .cloned()
        .filter(|key| matcher.fuzzy_match(key, partial).is_some())
        .collect::<Vec<_>>();

    // calling fuzzy_match again for a second time is fine cause it does caching
    keys.sort_by_key(|key| matcher.fuzzy_match(key, partial).unwrap_or(-1));

    keys.into_iter()
        .rev() // Reverse because higher score is better.
        .map(|key| serenity_prelude::AutocompleteChoice::new(key.to_string(), key))
        .collect()
}

use crate::{managers::lua_manager::CommandOption, Context, Error};

/// Create your own commands! (Requires something idk)
#[poise::command(
    slash_command,
    install_context = "Guild",
    interaction_context = "Guild",
    subcommands("create", "update", "delete", "download", "refresh"),
    subcommand_required,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn lua_command(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Creates a custom command.
#[poise::command(slash_command)]
pub async fn create(
    ctx: Context<'_>,
    command_name: String,
    description: String,
    params: Option<String>,
    lua_file: Attachment,
) -> Result<(), Error> {
    let params = params
        .map(|string| CommandOption::parse_string(&string))
        .unwrap_or(Ok(vec![]))?;

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
            response.text().await.unwrap_or_else(|err| err.to_string())
        )));
    }

    let lua_code = response.text().await?;

    let data = ctx.data();

    data.lua_manager
        .register_command(
            ctx.guild_id().unwrap(),
            command_name.clone(),
            description,
            params,
            lua_code,
            lua_file.filename,
        )
        .await?;

    ctx.send(
        CreateReply::default()
            .content(format!(
                "Successfully created custom command! Try it out using `/c {command_name}`",
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
    #[autocomplete = "autocomplete_command_name"] command_name: String,
    description: String,
    params: Option<String>,
    lua_file: Attachment,
) -> Result<(), Error> {
    let params = params
        .map(|string| CommandOption::parse_string(&string))
        .unwrap_or(Ok(vec![]))?;

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
            description,
            params,
            lua_code,
            lua_file.filename,
        )
        .await?;

    ctx.send(
        CreateReply::default()
            .content(format!(
                "Successfully updated the custom command! Try it out using /c {command_name}",
            ))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// Updates an existing custom command.
#[poise::command(slash_command)]
pub async fn delete(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_command_name"] command_name: String,
) -> Result<(), Error> {
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

/// Updates an existing custom command.
#[poise::command(slash_command)]
pub async fn download(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_command_name"] command_name: String,
) -> Result<(), Error> {
    let data = ctx.data();

    let guild_id = ctx.guild_id().unwrap();

    let guild_lua_data = data.lua_manager.get_guild_lua_data(guild_id).await;

    let mut commands_lock = guild_lua_data.lock().await;

    let commands = commands_lock.get_commands(&data.db).await?;

    let Some((command_info, _)) = commands.get(&command_name) else {
        ctx.send(CreateReply::default().content("no.").ephemeral(true))
            .await?;
        return Ok(());
    };

    ctx.send(
        CreateReply::default()
            .attachment(CreateAttachment::bytes(
                command_info.lua_code.clone(),
                command_info.filename.clone(),
            ))
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

    let guild_lua_data = data.lua_manager.get_guild_lua_data(guild_id).await;

    guild_lua_data
        .lock()
        .await
        .update_guild_commands(&data.db, ctx.http())
        .await?;

    ctx.send(
        CreateReply::default()
            .content("Successfully refreshed all custom commands.")
            .ephemeral(true),
    )
    .await?;

    Ok(())
}
