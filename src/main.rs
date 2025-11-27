#![feature(once_cell_try)]

mod commands;
mod managers;
mod tokens;
pub mod utils;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};

use managers::{
    cotd_manager::{cotd_manager_loop, CotdManager},
    currency_manager::CurrencyManager,
    db::SurrealClient,
    log_manager::{LogManager, LogSource, LogType},
    lua_manager::LuaManager,
    remind_manager::{remind_manager_loop, RemindManager},
    storage_manager::{storage_manager_loop, StorageManager},
};
use poise::serenity_prelude::{
    self as serenity, CreateInteractionResponse, CreateInteractionResponseMessage, FullEvent,
    Webhook,
};
use utils::IdType;

use crate::managers::{
    detector_manager::{detector_manager_loop, DetectorManager},
    join_order_manager::{join_order_manager_loop, JoinOrderManager, LightweightMember},
    log_manager::log_manager_loop,
    lua_manager::lua_manager_loop,
    profile_manager::{profile_manager_loop, ProfileManager},
    reaction_manager::{reaction_manager_loop, ReactionManager},
};

pub struct Data {
    started_loops: AtomicBool,
    storage_manager: Arc<StorageManager>,
    cotd_manager: Arc<CotdManager>,
    remind_manager: Arc<RemindManager>,
    detector_manager: Arc<DetectorManager>,
    reaction_manager: Arc<ReactionManager>,
    currency_manager: Arc<CurrencyManager>,
    profile_manager: Arc<ProfileManager>,
    join_order_manager: Arc<JoinOrderManager>,
    lua_manager: Arc<LuaManager>,
    log_manager: Arc<LogManager>,
    db: Arc<SurrealClient>,
} // User data, which is stored and accessible in all command invocations
pub struct Handler {} // User data, which is stored and accessible in all command invocations
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

async fn event_handler<'thing>(
    ctx: &serenity::Context,
    event: &FullEvent,
    framework: poise::FrameworkContext<'thing, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        FullEvent::Ready { data_about_bot } => {
            println!("Logged in as {}", data_about_bot.user.tag());
        }
        FullEvent::CacheReady { guilds } => {
            if !data.started_loops.load(Ordering::Relaxed) {
                println!("Caches are ready! Starting all the managers.");
                let arc_ctx = Arc::new(ctx.clone());
                storage_manager_loop(arc_ctx.clone(), data.storage_manager.clone());
                log_manager_loop(arc_ctx.clone(), data.log_manager.clone());
                cotd_manager_loop(
                    arc_ctx.clone(),
                    data.db.clone(),
                    data.cotd_manager.clone(),
                    data.log_manager.clone(),
                );
                remind_manager_loop(
                    arc_ctx.clone(),
                    data.remind_manager.clone(),
                    data.log_manager.clone(),
                );
                lua_manager_loop(data.lua_manager.clone());
                detector_manager_loop(data.detector_manager.clone());
                reaction_manager_loop(data.reaction_manager.clone());
                profile_manager_loop(data.profile_manager.clone());
                join_order_manager_loop(data.join_order_manager.clone());
                data.started_loops.swap(true, Ordering::Relaxed);
            }
            // TODO: Look through all relevant data and check if its still valid.
            // If a reminder for a user is in a guild the user is no longer in, remove them.
            // If a reaction role has an emoji, message or role that no longer exists, remove them.
            // If a folder about a guild still exists even though the bot is no longer in the guild, remove them.
            //
            // Also do these kinds of checks whenever communication to db hasn't worked.
            data.join_order_manager.join_orders.write().await.clear();
        }
        FullEvent::Resume { event: _ } => {
            data.join_order_manager.join_orders.write().await.clear();
        }
        FullEvent::GuildMemberAddition { new_member } => {
            if let Some(join_order) = data
                .join_order_manager
                .silent_get_join_order(new_member.guild_id)
                .await
            {
                join_order
                    .insert_member(LightweightMember {
                        id: new_member.user.id,
                        tag: new_member.user.tag(),
                        joined_at: new_member.joined_at,
                    })
                    .await;
            }
        }
        FullEvent::GuildMemberRemoval {
            guild_id,
            user,
            member_data_if_available: _,
        } => {
            if let Some(join_order) = data
                .join_order_manager
                .silent_get_join_order(*guild_id)
                .await
            {
                join_order.remove_member(user.id).await;
            }
        }
        FullEvent::GuildMemberUpdate {
            old_if_available,
            new: _,
            event,
        } => {
            if let Some(join_order) = data
                .join_order_manager
                .silent_get_join_order(event.guild_id)
                .await
            {
                if let Some(old) = old_if_available {
                    if old.user.tag() == event.user.tag() && old.joined_at == Some(event.joined_at)
                    {
                        return Ok(());
                    }
                }
                join_order
                    .update_member(LightweightMember {
                        id: event.user.id,
                        tag: event.user.tag(),
                        joined_at: Some(event.joined_at),
                    })
                    .await;
            }
        }
        FullEvent::Message { new_message } => {
            if let Err(err) = data.detector_manager.on_message(ctx, new_message).await {
                let id;

                if let Some(guild_id) = new_message.guild_id {
                    id = IdType::GuildId(guild_id)
                } else {
                    id = IdType::UserId(new_message.author.id)
                }

                data.log_manager
                    .add_log(
                        id,
                        err.to_string(),
                        LogType::Warning,
                        LogSource::MessageDetector,
                    )
                    .await;
            }
        }
        FullEvent::ReactionAdd { add_reaction } => {
            let res = data
                .reaction_manager
                .reaction_add_event(ctx, add_reaction, framework.bot_id)
                .await;

            if let Err(err) = res {
                let id;

                if let Some(guild_id) = add_reaction.guild_id {
                    id = IdType::GuildId(guild_id)
                } else {
                    let Some(user_id) = add_reaction.user_id else {
                        return Ok(());
                    };
                    id = IdType::UserId(user_id)
                }

                let _ = data
                    .log_manager
                    .add_log(
                        id,
                        err.to_string(),
                        LogType::Warning,
                        LogSource::ReactionRole,
                    )
                    .await;
            }
        }
        FullEvent::ReactionRemove { removed_reaction } => {
            let res = data
                .reaction_manager
                .reaction_remove_event(ctx, removed_reaction, framework.bot_id)
                .await;

            if let Err(err) = res {
                let id;

                if let Some(guild_id) = removed_reaction.guild_id {
                    id = IdType::GuildId(guild_id)
                } else {
                    let Some(user_id) = removed_reaction.user_id else {
                        return Ok(());
                    };
                    id = IdType::UserId(user_id)
                }

                let _ = data
                    .log_manager
                    .add_log(
                        id,
                        err.to_string(),
                        LogType::Warning,
                        LogSource::ReactionRole,
                    )
                    .await;
            }
        }
        FullEvent::ReactionRemoveAll {
            channel_id,
            removed_from_message_id,
        } => {
            let channel = channel_id.to_channel(ctx).await?;
            let Some(guild) = channel.guild() else {
                return Ok(());
            };

            let message_id = *removed_from_message_id;

            let guild_id = guild.guild_id;
            let res = data
                .reaction_manager
                .reaction_remove_all(guild_id, message_id)
                .await;

            if let Err(err) = res {
                let id = IdType::GuildId(guild_id);

                let _ = data
                    .log_manager
                    .add_log(
                        id,
                        err.to_string(),
                        LogType::Warning,
                        LogSource::ReactionRole,
                    )
                    .await;
            }
        }
        FullEvent::ReactionRemoveEmoji { removed_reactions } => {
            let res = data
                .reaction_manager
                .reaction_remove_emoji(removed_reactions)
                .await;

            if let Err(err) = res {
                let id;

                if let Some(guild_id) = removed_reactions.guild_id {
                    id = IdType::GuildId(guild_id)
                } else {
                    let Some(user_id) = removed_reactions.user_id else {
                        return Ok(());
                    };
                    id = IdType::UserId(user_id)
                }

                let _ = data
                    .log_manager
                    .add_log(
                        id,
                        err.to_string(),
                        LogType::Warning,
                        LogSource::ReactionRole,
                    )
                    .await;
            }
        }
        FullEvent::InteractionCreate { interaction } => match interaction {
            serenity::Interaction::Command(command_interaction) => {
                if let Some(guild_id) = command_interaction.data.guild_id {
                    let result = data
                        .lua_manager
                        .execute_command(guild_id, command_interaction.clone())
                        .await;

                    match result {
                        Ok(did_reply) => {
                            if !did_reply {
                                command_interaction
                                        .create_response(
                                            ctx,
                                            CreateInteractionResponse::Message(
                                                CreateInteractionResponseMessage::new()
                                                    .content("This command has executed, but didn't send a reply."),
                                            ),
                                        )
                                        .await?;
                            }
                        }
                        Err(err) => {
                            let _ = data
                                .log_manager
                                .add_log(
                                    IdType::GuildId(guild_id),
                                    err.to_string(),
                                    LogType::Error,
                                    LogSource::Lua,
                                )
                                .await;

                            command_interaction
                                .create_response(
                                    ctx,
                                    CreateInteractionResponse::Message(
                                        CreateInteractionResponseMessage::new().content(format!(
                                            "An error occurred while executing this command: {err}"
                                        )),
                                    ),
                                )
                                .await?
                        }
                    }
                }
            }
            _ => {}
        },
        _ => {}
    }
    Ok(())
}
#[tokio::main]
async fn main() {
    println!("Starting bot...");

    let bot_settings = tokens::get_bot_settings();

    let db = Arc::new(SurrealClient::new(bot_settings.surrealdb));

    let storage_manager = Arc::new(StorageManager::new(bot_settings.temp_data_directory).await);

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands::get_commands(),
            event_handler: |ctx: &serenity::Context,
                            event: &serenity::FullEvent,
                            framework,
                            data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                println!("Registering commands...");
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                let arc_ctx = Arc::new(ctx.clone());

                let log_manager = Arc::new(LogManager::new(
                    storage_manager.clone(),
                    bot_settings.owner_user_ids,
                    arc_ctx.clone(),
                ));

                Ok(Data {
                    cotd_manager: Arc::new(CotdManager::new(db.clone())),
                    remind_manager: Arc::new(RemindManager::new(db.clone())),
                    detector_manager: Arc::new(DetectorManager::new(db.clone())),
                    reaction_manager: Arc::new(ReactionManager::new(db.clone())),
                    lua_manager: Arc::new(LuaManager::new(
                        db.clone(),
                        log_manager.clone(),
                        arc_ctx,
                    )),
                    currency_manager: Arc::new(
                        CurrencyManager::new(bot_settings.open_exchange_rates_token).await,
                    ),
                    profile_manager: Arc::new(ProfileManager::new()),
                    join_order_manager: Arc::new(JoinOrderManager::new()),
                    log_manager,
                    storage_manager,
                    started_loops: AtomicBool::new(false),
                    db,
                })
            })
        })
        .build();

    let intents = serenity::GatewayIntents::GUILDS
        | serenity::GatewayIntents::GUILD_MESSAGES
        | serenity::GatewayIntents::GUILD_MEMBERS
        | serenity::GatewayIntents::GUILD_MESSAGE_REACTIONS
        | serenity::GatewayIntents::DIRECT_MESSAGES
        | serenity::GatewayIntents::MESSAGE_CONTENT;

    let client = serenity::ClientBuilder::new(bot_settings.discord_token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap()
}
