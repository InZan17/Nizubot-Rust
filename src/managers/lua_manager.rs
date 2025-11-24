use std::{
    collections::HashMap,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Weak,
    },
    time::Duration,
};

use mlua::{Function, IntoLua, Lua, UserData, UserDataMethods, VmState};
use poise::serenity_prelude::{
    self, CacheHttp, CommandDataOptionValue, CommandInteraction, CommandOptionType, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage, GuildId,
    Http,
};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{Mutex, Notify, RwLock},
    time::sleep,
};

use crate::{
    managers::lua_manager::{
        self,
        serde_and_lua::{lua_to_serde, serde_to_lua},
    },
    utils::{TtlMap, TtlMapWithArcTokioMutex},
    Error,
};

use super::db::SurrealClient;

pub mod serde_and_lua;

#[derive(Serialize, Deserialize, Clone)]
pub struct CommandOption {
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}

impl CommandOption {
    pub fn parse_string(string: &str) -> Result<Vec<Self>, Error> {
        let mut options = Vec::new();
        let params = string.split(';');
        for param in params {
            if param.is_empty() {
                continue;
            }

            let properties = param.split(':');

            let mut param_name = None;
            let mut param_type = None;
            let mut description = None;
            let mut required = true;

            for property in properties {
                let Some((property_name, property_value)) = property.split_once('=') else {
                    return Err("Your parameters are incorrectly formatted.".into());
                };

                match property_name {
                    "name" => param_name = Some(property_value),
                    "type" => param_type = Some(property_value),
                    "description" => description = Some(property_value),
                    "required" => {
                        let lower = property_value.to_lowercase();
                        if lower == "true".to_string() {
                            required = true;
                        } else if lower == "false".to_string() {
                            required = false;
                        } else {
                            return Err(
                                format!("The value for \"required\" must be either \"true\" or \"false\". You provided \"{property_value}\".").into()
                            );
                        }
                    }
                    _ => return Err(format!("\"{property_name}\" is not a valid property.").into()),
                }
            }

            let Some(param_name) = param_name else {
                return Err(
                    format!("\"name\" property is missing on one of the parameters.").into(),
                );
            };

            let Some(param_type) = param_type else {
                return Err(
                    format!("\"type\" property is missing on one of the parameters.").into(),
                );
            };

            options.push(CommandOption {
                kind: param_type.to_string(),
                name: param_name.to_string(),
                description: description.map(|str| str.to_string()),
                required,
            });
        }

        Ok(options)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LuaCommandInfo {
    pub lua_code: String,
    pub filename: String,
    pub description: String,
    pub options: Vec<CommandOption>,
}

pub struct DataStore {
    guild_id: GuildId,
    data_store_name: String,
    db: Arc<SurrealClient>,
    pub data: Option<HashMap<String, serde_json::Value>>,
}

impl DataStore {
    pub fn new(guild_id: GuildId, data_store_name: String, db: Arc<SurrealClient>) -> Self {
        Self {
            guild_id,
            data_store_name,
            db,
            data: None,
        }
    }

    pub async fn get_data(&mut self) -> Result<&mut HashMap<String, serde_json::Value>, Error> {
        let data_mut = &mut self.data;
        match data_mut {
            Some(data) => return Ok(data),
            None => {
                let fetched_data = self
                    .db
                    .get_data_store(self.guild_id, &self.data_store_name)
                    .await?;

                *data_mut = Some(fetched_data);
                return Ok(data_mut.as_mut().unwrap());
            }
        }
    }

    pub async fn get(&mut self, key: &str) -> Result<serde_json::Value, Error> {
        let data = self.get_data().await?;
        Ok(data.get(key).unwrap_or(&serde_json::Value::Null).clone())
    }

    pub async fn set(
        &mut self,
        key: String,
        value: serde_json::Value,
    ) -> Result<serde_json::Value, Error> {
        let guild_id = self.guild_id;
        let data_store_name = self.data_store_name.clone();
        let db = self.db.clone();
        let data = self.get_data().await?;
        db.set_data_store_value(guild_id, &data_store_name, &key, &value)
            .await?;

        Ok(data.insert(key, value).unwrap_or(serde_json::Value::Null))
    }
}

pub struct DataStoreWrapper(Arc<Mutex<DataStore>>);

impl From<Arc<Mutex<DataStore>>> for DataStoreWrapper {
    fn from(value: Arc<Mutex<DataStore>>) -> Self {
        Self(value)
    }
}

impl Deref for DataStoreWrapper {
    type Target = Arc<Mutex<DataStore>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UserData for DataStoreWrapper {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {}

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method("get", async |lua, this, key: String| {
            let mut lock = this.lock().await;

            let serde_value = lock
                .get(&key)
                .await
                .map_err(|err| mlua::Error::runtime(err))?;

            let lua_value = serde_to_lua(serde_value, &lua)?;

            Ok(lua_value)
        });

        methods.add_async_method(
            "set",
            async |lua, this, (key, lua_value): (String, mlua::Value)| {
                let serde_value =
                    lua_to_serde(lua_value).map_err(|err| mlua::Error::runtime(err))?;

                let mut lock = this.lock().await;

                let previous_serde_value = lock
                    .set(key, serde_value)
                    .await
                    .map_err(|err| mlua::Error::runtime(err))?;

                let previous_lua_value = serde_to_lua(previous_serde_value, &lua)?;

                Ok(previous_lua_value)
            },
        );
    }
}

pub struct LuaAppData {
    pub lua_manager: Weak<LuaManager>,
    pub guild_id: GuildId,
}

pub struct GuildLuaData {
    lua: Option<Lua>,
    guild_id: GuildId,
    stop_notify: Arc<Notify>,
    stop_bool: Arc<AtomicBool>,
    lua_manager: Weak<LuaManager>,
    pub commands: Option<HashMap<String, (LuaCommandInfo, Option<Function>)>>,
}

impl GuildLuaData {
    pub fn new(guild_id: GuildId, lua_manager: Arc<LuaManager>) -> Self {
        Self {
            lua: None,
            guild_id,
            stop_notify: Arc::new(Notify::new()),
            stop_bool: Arc::new(AtomicBool::new(false)),
            lua_manager: Arc::downgrade(&lua_manager),
            commands: None,
        }
    }

    pub fn stop_execution(&mut self) {
        self.stop_bool.store(true, Ordering::Relaxed);
        self.stop_notify.notify_waiters();
        self.lua = None;
    }

    pub fn allow_execution(&mut self) {
        self.stop_bool.store(false, Ordering::Relaxed);
    }

    pub fn get_lua(&mut self) -> Result<Lua, mlua::Error> {
        if let Some(lua) = &self.lua {
            return Ok(lua.clone());
        };

        let lua = Lua::new();

        let stop_bool = self.stop_bool.clone();
        let count = Arc::new(AtomicU64::new(0));

        let yield_frequency = 50;

        lua.set_interrupt(move |_lua| {
            if stop_bool.load(Ordering::SeqCst)
                || count.fetch_add(1, Ordering::Relaxed) % yield_frequency == 0
            {
                Ok(VmState::Yield)
            } else {
                Ok(VmState::Continue)
            }
        });

        // For debug purposes only.
        lua.globals().set(
            "wait",
            lua.create_async_function(async |lua, wait: f32| {
                sleep(Duration::from_secs_f32(wait)).await;
                Ok(())
            })?,
        )?;

        let lua_manager = self.lua_manager.clone();
        let guild_id = self.guild_id;

        lua.globals().set(
            "get_data_store",
            lua.create_async_function(move |_lua, data_store_name| {
                let lua_manager = lua_manager.clone();
                async move {
                    let Some(lua_manager) = lua_manager.upgrade() else {
                        return Err(mlua::Error::runtime("Failed to get LuaManager."));
                    };

                    Ok(DataStoreWrapper::from(
                        lua_manager.get_data_store(guild_id, data_store_name).await,
                    ))
                }
            })?,
        )?;

        lua.sandbox(true)?;

        self.lua = Some(lua.clone());

        Ok(lua)
    }

    pub async fn get_commands(
        &mut self,
        db: &SurrealClient,
    ) -> Result<&mut HashMap<String, (LuaCommandInfo, Option<Function>)>, Error> {
        let commands_mut = &mut self.commands;
        match commands_mut {
            Some(commands) => return Ok(commands),
            None => {
                let fetched_commands = db.get_all_guild_lua_commands(self.guild_id).await?;
                let mapped_commands = fetched_commands
                    .into_iter()
                    .map(|(k, v)| (k, (v, None)))
                    .collect();

                *commands_mut = Some(mapped_commands);
                return Ok(commands_mut.as_mut().unwrap());
            }
        }
    }

    pub async fn add_or_replace_command(
        &mut self,
        command_name: String,
        lua_command_info: LuaCommandInfo,
        db: &SurrealClient,
    ) -> Result<(), Error> {
        let guild_id = self.guild_id;
        let commands = self.get_commands(db).await?;
        db.add_guild_lua_command(&command_name, &lua_command_info, guild_id)
            .await?;
        commands.insert(command_name, (lua_command_info, None));
        Ok(())
    }

    pub async fn delete_command(
        &mut self,
        command_name: String,
        db: &SurrealClient,
    ) -> Result<(), Error> {
        let guild_id = self.guild_id;
        let commands = self.get_commands(db).await?;
        db.remove_guild_lua_command(guild_id, &command_name).await?;
        commands.remove(&command_name);
        Ok(())
    }

    pub async fn update_guild_commands(
        &mut self,
        db: &SurrealClient,
        http: &Http,
    ) -> Result<(), Error> {
        let guild_id = self.guild_id;
        let commands = self.get_commands(db).await?;
        if commands.is_empty() {
            let guild_commands = http.get_guild_commands(guild_id).await?;
            if let Some(command) = guild_commands.iter().find(|command| &command.name == "c") {
                http.delete_guild_command(guild_id, command.id).await?;
                return Ok(());
            };
        }

        let mut create_command = CreateCommand::new("c").description("Custom commands");
        for (command_name, (command_info, _)) in commands {
            let mut sub_command = CreateCommandOption::new(
                CommandOptionType::SubCommand,
                command_name,
                command_info.description.clone(),
            );

            for option in command_info.options.iter() {
                let description = option.description.clone().unwrap_or(option.name.clone());

                sub_command = sub_command.add_sub_option(
                    CreateCommandOption::new(
                        get_command_option_type_from_str(&option.kind)?,
                        option.name.clone(),
                        description,
                    )
                    .required(option.required),
                );
            }

            create_command = create_command.add_option(sub_command);
        }

        http.create_guild_command(guild_id, &create_command).await?;

        Ok(())
    }

    pub async fn get_command_function(
        &mut self,
        command_name: &str,
        db: &SurrealClient,
    ) -> Result<(Function, Lua, Arc<Notify>), Error> {
        let lua = self.get_lua()?;

        let commands = self.get_commands(db).await?;

        let Some((command_info, command_function)) = commands.get_mut(command_name) else {
            return Err(format!("No command with name: {command_name}").into());
        };

        if let Some(function) = command_function {
            return Ok((function.clone(), lua, self.stop_notify.clone()));
        };

        let function: Function = lua
            .load(&command_info.lua_code)
            .set_name(format!(
                "={} (Command: {command_name})",
                command_info.filename
            ))
            .eval_async()
            .await?;

        *command_function = Some(function.clone());

        return Ok((function, lua, self.stop_notify.clone()));
    }
}

pub struct LuaManager {
    db: Arc<SurrealClient>,
    /// Holds data such as the guild commands and the lua instance and functions.
    ///
    /// GuildLuaData is inside of an Arc so that the RwLock gets locked as little as possible.
    /// This is also fine because GuildLuaData uses interior mutability.
    ///
    /// As long as the Arc doesn't get saved anywhere / anything uses it for longer than the duration of the TtlMap,
    /// everything will be fine. The concern otherwise would be that the entry gets removed,
    /// and something still has an Arc from that entry and end up doing things that wont be properly saved.
    pub guild_data: RwLock<TtlMap<GuildId, Arc<Mutex<GuildLuaData>>>>,
    pub data_stores: RwLock<HashMap<GuildId, Mutex<TtlMapWithArcTokioMutex<String, DataStore>>>>,
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
        Self {
            db,
            arc_ctx,
            guild_data: RwLock::new(TtlMap::new(Duration::from_secs(60 * 60))),
            data_stores: RwLock::new(HashMap::new()),
        }
    }

    /// NOTE: It is VERY IMPORTANT that you do not store this Arc anywhere for long term use!
    pub async fn get_guild_lua_data(
        self: &Arc<Self>,
        guild_id: GuildId,
    ) -> Arc<Mutex<GuildLuaData>> {
        if let Some(guild_lua_data) = self.guild_data.read().await.get(&guild_id).cloned() {
            return guild_lua_data;
        }

        let mut guild_data_mut = self.guild_data.write().await;
        if let Some(guild_lua_data) = guild_data_mut.get(&guild_id).cloned() {
            return guild_lua_data;
        }

        let guild_lua_data = Arc::new(Mutex::new(GuildLuaData::new(guild_id, self.clone())));

        guild_data_mut.insert(guild_id, guild_lua_data.clone());

        guild_lua_data
    }

    pub async fn get_data_store(
        &self,
        guild_id: GuildId,
        data_store_name: String,
    ) -> Arc<Mutex<DataStore>> {
        fn get_or_insert_data_store(
            map: &mut TtlMapWithArcTokioMutex<String, DataStore>,
            guild_id: GuildId,
            data_store_name: String,
            db: Arc<SurrealClient>,
        ) -> Arc<Mutex<DataStore>> {
            if let Some(data_store) = map.get(&data_store_name) {
                return data_store;
            };

            return map.insert(
                data_store_name.clone(),
                DataStore::new(guild_id, data_store_name, db),
            );
        }

        if let Some(data_stores_map) = self.data_stores.read().await.get(&guild_id) {
            let mut data_stores_map_lock = data_stores_map.lock().await;
            return get_or_insert_data_store(
                &mut data_stores_map_lock,
                guild_id,
                data_store_name,
                self.db.clone(),
            );
        }

        let mut data_stores_mut = self.data_stores.write().await;
        if let Some(data_stores_map) = data_stores_mut.get(&guild_id) {
            let mut data_stores_map_lock = data_stores_map.lock().await;
            return get_or_insert_data_store(
                &mut data_stores_map_lock,
                guild_id,
                data_store_name,
                self.db.clone(),
            );
        }

        let mut map = TtlMapWithArcTokioMutex::new(Duration::from_secs(60 * 60));

        let data_store =
            get_or_insert_data_store(&mut map, guild_id, data_store_name, self.db.clone());

        data_stores_mut.insert(guild_id, Mutex::new(map));

        return data_store;
    }

    /// Registers a command and updates the guild command, but only if the guild hasnt reached the limit or if a command with a similar name doesn't exist.
    pub async fn register_command(
        self: &Arc<Self>,
        guild_id: GuildId,
        command_name: String,
        description: String,
        options: Vec<CommandOption>,
        lua_code: String,
        filename: String,
    ) -> Result<(), Error> {
        let guild_info = self.get_guild_lua_data(guild_id).await;
        let mut locked_guild_info = guild_info.lock().await;

        let commands = locked_guild_info.get_commands(&self.db).await?;

        if commands.len() >= 25 {
            return Err("You may only have up to 25 custom commands.".into());
        }

        if commands.contains_key(&command_name) {
            return Err(format!("A command with the name {command_name} already exists. Try updating or removing the command instead.").into());
        }

        // Make sure the provided code is valid lua code.
        self.try_parse_code(&lua_code)?;

        let lua_command_info = LuaCommandInfo {
            lua_code,
            filename,
            description,
            options,
        };

        locked_guild_info
            .add_or_replace_command(command_name, lua_command_info, &self.db)
            .await?;
        locked_guild_info
            .update_guild_commands(&self.db, self.arc_ctx.http())
            .await?;

        Ok(())
    }

    /// Updates a command and updates the guild command, but only if the command it will update exists.
    pub async fn update_command(
        self: &Arc<Self>,
        guild_id: GuildId,
        command_name: String,
        description: String,
        options: Vec<CommandOption>,
        lua_code: String,
        filename: String,
    ) -> Result<(), Error> {
        let guild_info = self.get_guild_lua_data(guild_id).await;
        let mut locked_guild_info = guild_info.lock().await;

        let commands = locked_guild_info.get_commands(&self.db).await?;

        if !commands.contains_key(&command_name) {
            return Err(format!("A command with the name {command_name} doesn't exists. Try creating a new command instead.").into());
        }

        // Make sure the provided code is valid lua code.
        self.try_parse_code(&lua_code)?;

        let lua_command_info = LuaCommandInfo {
            lua_code,
            filename,
            description,
            options,
        };

        locked_guild_info
            .add_or_replace_command(command_name, lua_command_info, &self.db)
            .await?;
        locked_guild_info
            .update_guild_commands(&self.db, self.arc_ctx.http())
            .await?;
        Ok(())
    }

    pub fn try_parse_code(&self, lua_code: &str) -> Result<(), Error> {
        let lua = Lua::new();

        let chunk = lua.load(lua_code);

        chunk.into_function()?;

        Ok(())
    }

    /// Deletes a command and updates the guild command, but only if the command it will delete exists.
    pub async fn delete_command(
        self: &Arc<Self>,
        guild_id: GuildId,
        command_name: String,
    ) -> Result<(), Error> {
        let guild_info = self.get_guild_lua_data(guild_id).await;
        let mut locked_guild_info = guild_info.lock().await;

        let commands = locked_guild_info.get_commands(&self.db).await?;

        if !commands.contains_key(&command_name) {
            return Err(format!("A command with the name {command_name} doesn't exists. If you can still see the command on discord and it's still there after a refresh, try running the refresh command.").into());
        };

        locked_guild_info
            .delete_command(command_name, &self.db)
            .await?;
        locked_guild_info
            .update_guild_commands(&self.db, self.arc_ctx.http())
            .await?;

        Ok(())
    }

    pub async fn execute_command(
        self: &Arc<Self>,
        guild_id: GuildId,
        command_interaction: CommandInteraction,
    ) -> Result<bool, Error> {
        if command_interaction.data.options.len() != 1 {
            return Err("Unexpected command options length.".into());
        }

        println!("{:?}", command_interaction.data);

        let command_option = &command_interaction.data.options[0];
        let command_name = &command_option.name;

        let guild_info = self.get_guild_lua_data(guild_id).await;

        let mut locked_guild_info = guild_info.lock().await;

        let (function, lua, notify) = locked_guild_info
            .get_command_function(command_name, &self.db)
            .await?;

        drop(locked_guild_info);
        drop(guild_info);

        let CommandDataOptionValue::SubCommand(sub_command_args) = &command_option.value else {
            return Err("Option wasn't a subcommand.".into());
        };

        let command_args_lua = lua.create_table()?;

        for argument in sub_command_args.iter() {
            let argument_value = match &argument.value {
                CommandDataOptionValue::Boolean(v) => mlua::Value::Boolean(*v),
                CommandDataOptionValue::Integer(v) => mlua::Value::Integer(*v as _),
                CommandDataOptionValue::Number(v) => mlua::Value::Number(*v),
                CommandDataOptionValue::String(v) => v.clone().into_lua(&lua)?,
                _ => return Err("Unsupported command option type.".into()),
            };
            command_args_lua.set(argument.name.clone(), argument_value)?;
        }

        let context_container = ContextContainer {
            arc_ctx: self.arc_ctx.clone(),
            command_interaction,
            sent_reply: false,
        };

        let context_container_userdata = lua.create_userdata(context_container)?;

        let _result = tokio::select! {
            _ = notify.notified() => {
                Err(mlua::Error::runtime("Operation cancelled by a higher being."))
            }
            result = function
            .call_async::<mlua::Value>((&context_container_userdata, command_args_lua))
            => result,
        }?;

        let context_container = context_container_userdata.borrow::<ContextContainer>()?;

        let sent_reply = context_container.sent_reply;

        drop(context_container);
        let _ = context_container_userdata.destroy();

        return Ok(sent_reply);
    }
}

pub fn lua_manager_loop(lua_manager: Arc<LuaManager>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30 * 60)).await;
            {
                let mut guild_data_write = lua_manager.guild_data.write().await;
                guild_data_write.clear_expired();
            }
            {
                let mut data_store_write = lua_manager.data_stores.write().await;
                data_store_write.retain(|_guild_id, ttl_map| {
                    let Ok(mut lock) = ttl_map.try_lock() else {
                        // This should always successfully lock.
                        // I have a write lock on data_stores which makes it so I'm the only holder.
                        return true;
                    };
                    lock.clear_expired();
                    !lock.is_empty()
                });
            }
        }
    });
}
