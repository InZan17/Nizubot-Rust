use std::sync::Arc;

use mlua::{FromLua, IntoLua, Lua, UserData, UserDataMethods};
use poise::serenity_prelude::{
    self, CacheHttp, CommandDataOptionValue, CommandInteraction, CommandOptionType, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage, GuildId,
};
use serde::{Deserialize, Serialize};

use crate::Error;

use super::db::SurrealClient;

#[derive(Serialize, Deserialize, Clone)]
pub struct CommandOption {
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}

impl FromLua for CommandOption {
    fn from_lua(value: mlua::Value, _lua: &Lua) -> mlua::Result<Self> {
        let Some(table) = value.as_table() else {
            return Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "CommandOption".to_string(),
                message: Some("expected table".to_string()),
            });
        };
        Ok(CommandOption {
            kind: table.get("type")?,
            name: table.get("name")?,
            description: table.get("description")?,
            required: table.get::<Option<_>>("required")?.unwrap_or(false),
        })
    }
}

pub struct LuaCommandData {
    pub run: mlua::Function,
    pub description: Option<String>,
    pub options: Vec<CommandOption>,
}

impl FromLua for LuaCommandData {
    fn from_lua(value: mlua::Value, _lua: &Lua) -> mlua::Result<Self> {
        let Some(table) = value.as_table() else {
            return Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "LuaCommandData".to_string(),
                message: Some("expected table".to_string()),
            });
        };
        Ok(LuaCommandData {
            run: table.get("run")?,
            description: table.get("description")?,
            options: table.get::<Option<_>>("options")?.unwrap_or(Vec::new()),
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LuaCommandInfo {
    pub lua_code: String,
    pub filename: String,
    pub sub_command_data: SubCommand,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SubCommand {
    pub name: String,
    pub description: String,
    pub options: Vec<CommandOption>,
}

pub struct LuaManager {
    db: Arc<SurrealClient>,
    arc_ctx: Arc<serenity_prelude::Context>,
}

pub fn response(content: String) -> CreateInteractionResponse {
    CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content(content))
}

pub fn get_command_option_type_from_str(value: &str) -> Result<CommandOptionType, String> {
    match value {
        "bool" | "boolean" => Ok(CommandOptionType::Boolean),
        "integer" => Ok(CommandOptionType::Integer),
        "number" => Ok(CommandOptionType::Number),
        "string" => Ok(CommandOptionType::String),
        _ => Err(format!("\"{value}\" is not a valid option type.")),
    }
}

pub struct ContextContainer {
    arc_ctx: Arc<serenity_prelude::Context>,
    command_interaction: CommandInteraction,
    sent_reply: bool,
}

impl UserData for ContextContainer {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method_mut("reply", |_, mut this, content: String| async move {
            this.command_interaction
                .create_response(&this.arc_ctx, response(content))
                .await
                .map_err(|err| mlua::Error::external(err))?;
            this.sent_reply = true;
            Ok(())
        });
    }
}

impl LuaManager {
    pub fn new(db: Arc<SurrealClient>, arc_ctx: Arc<serenity_prelude::Context>) -> Self {
        Self { db, arc_ctx }
    }

    /// Registers a command and updates the guild command, but only if the guild hasnt reached the limit or if a command with a similar name doesn't exist.
    pub async fn register_command(
        &self,
        guild_id: GuildId,
        command_name: String,
        lua_code: String,
        filename: String,
    ) -> Result<(), Error> {
        let command_infos = self.db.get_all_guild_lua_commands(guild_id).await?;

        if command_infos.len() >= 25 {
            return Err("You may only have up to 25 custom commands.".into());
        }

        if command_infos
            .iter()
            .any(|c| c.sub_command_data.name == command_name)
        {
            return Err(format!("A command with the name {command_name} already exists. Try updating or removing the command instead.").into());
        }

        return self
            .register_command_unchecked(guild_id, command_name, lua_code, filename, command_infos)
            .await;
    }

    /// Updates a command and updates the guild command, but only if the command it will update exists.
    pub async fn update_command(
        &self,
        guild_id: GuildId,
        command_name: String,
        lua_code: String,
        filename: String,
    ) -> Result<(), Error> {
        let mut command_infos = self.db.get_all_guild_lua_commands(guild_id).await?;

        let old_len = command_infos.len();

        command_infos.retain(|command_info| command_info.sub_command_data.name != command_name);

        if old_len == command_infos.len() {
            return Err(format!("A command with the name {command_name} doesn't exists. Try creating a new command instead.").into());
        }

        return self
            .register_command_unchecked(guild_id, command_name, lua_code, filename, command_infos)
            .await;
    }

    /// Registers a command and updates the guild command, but doesnt check if the guild has reached max commands, or if a command with same name exists.
    ///
    /// It's very important to make sure command_name doesnt exist inside command_infos before calling this function.
    pub async fn register_command_unchecked(
        &self,
        guild_id: GuildId,
        command_name: String,
        lua_code: String,
        filename: String,
        mut command_infos: Vec<LuaCommandInfo>,
    ) -> Result<(), Error> {
        let lua = Lua::new();

        lua.sandbox(true)?;

        let chunk = lua.load(&lua_code).set_name(format!("={}", filename));
        let command_data = chunk.eval::<LuaCommandData>()?;

        let lua_command_info = LuaCommandInfo {
            lua_code,
            filename,
            sub_command_data: SubCommand {
                name: command_name,
                description: command_data
                    .description
                    .unwrap_or("Custom command".to_string()),
                options: command_data.options,
            },
        };

        command_infos.push(lua_command_info.clone());

        self.update_guild_commands(guild_id, command_infos).await?;

        self.db
            .add_guild_lua_command(&lua_command_info, guild_id)
            .await?;

        Ok(())
    }

    /// Deletes a command and updates the guild command, but only if the command it will delete exists.
    pub async fn delete_command(
        &self,
        guild_id: GuildId,
        command_name: String,
    ) -> Result<(), Error> {
        let mut command_infos = self.db.get_all_guild_lua_commands(guild_id).await?;

        let Some(index) = command_infos
            .iter()
            .position(|c| c.sub_command_data.name == command_name)
        else {
            return Err(format!("A command with the name {command_name} doesn't exists. If you can still see the command on discord and it's still there after a refresh, try running the refresh command.").into());
        };

        command_infos.remove(index);

        self.db.remove_guild_lua_command(guild_id, index).await?;

        return self.update_guild_commands(guild_id, command_infos).await;
    }

    /// Updates the guild commands inside of discord.
    pub async fn update_guild_commands(
        &self,
        guild_id: GuildId,
        command_infos: Vec<LuaCommandInfo>,
    ) -> Result<(), Error> {
        let mut create_command = CreateCommand::new("c").description("Custom commands");
        for command_info in command_infos {
            let mut sub_command = CreateCommandOption::new(
                CommandOptionType::SubCommand,
                command_info.sub_command_data.name,
                command_info.sub_command_data.description,
            );

            for option in command_info.sub_command_data.options {
                sub_command = sub_command.add_sub_option(
                    CreateCommandOption::new(
                        get_command_option_type_from_str(&option.kind)?,
                        option.name,
                        option.description.unwrap_or("field".to_string()),
                    )
                    .required(option.required),
                );
            }

            create_command = create_command.add_option(sub_command);
        }

        self.arc_ctx
            .http()
            .create_guild_command(guild_id, &create_command)
            .await?;

        Ok(())
    }

    pub async fn execute_command(
        &self,
        guild_id: GuildId,
        command_interaction: CommandInteraction,
    ) -> Result<bool, Error> {
        // TODO: make one which takes a name parameter and returns only 1 thing.
        let command_infos = self.db.get_all_guild_lua_commands(guild_id).await?;

        let mut lua_command = None;

        println!("{:?}", command_interaction.data);

        if command_interaction.data.options.len() != 1 {
            return Err("Unexpected command options length.".into());
        }

        let sub_command_option = &command_interaction.data.options[0];
        let sub_command_name = &sub_command_option.name;

        for command_info in command_infos {
            if command_info.sub_command_data.name == *sub_command_name {
                lua_command = Some(command_info);
                break;
            }
        }

        let Some(lua_command) = lua_command else {
            return Err("Couldn't find command info.".into());
        };

        let CommandDataOptionValue::SubCommand(sub_command_args) = &sub_command_option.value else {
            return Err("Option wasn't a subcommand.".into());
        };

        let lua = Lua::new();

        let command_args_lua = lua.create_table()?;

        for argument in sub_command_args.iter() {
            let argument_value = match &argument.value {
                CommandDataOptionValue::Boolean(v) => mlua::Value::Boolean(*v),
                CommandDataOptionValue::Integer(v) => mlua::Value::Integer(*v as i32),
                CommandDataOptionValue::Number(v) => mlua::Value::Number(*v),
                CommandDataOptionValue::String(v) => v.clone().into_lua(&lua)?,
                _ => return Err("Unsupported command option type.".into()),
            };
            command_args_lua.set(argument.name.clone(), argument_value)?;
        }

        lua.sandbox(true)?;

        let chunk = lua
            .load(&lua_command.lua_code)
            .set_name(format!("={}", lua_command.filename));

        let command_data = chunk.eval::<LuaCommandData>()?;

        let context_container = ContextContainer {
            arc_ctx: self.arc_ctx.clone(),
            command_interaction,
            sent_reply: false,
        };

        let context_container_userdata = lua.create_userdata(context_container)?;

        command_data
            .run
            .call_async::<mlua::Value>((&context_container_userdata, command_args_lua))
            .await?;

        let context_container = context_container_userdata.borrow::<ContextContainer>()?;

        return Ok(context_container.sent_reply);
    }
}
