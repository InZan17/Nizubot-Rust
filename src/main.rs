#![feature(downcast_unchecked)]
#![feature(get_mut_unchecked)]
#![feature(slice_pattern)]

mod commands;
pub mod give_up_serialize;
mod managers;
mod tokens;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use managers::{
    cotd_manager::{cotd_manager_loop, CotdManager},
    currency_manager::CurrencyManager,
    db::SurrealClient,
    remind_manager::{remind_manager_loop, RemindManager},
    storage_manager::{storage_manager_loop, StorageManager},
};
use poise::{
    framework,
    serenity_prelude::{self as serenity},
    Event, ReplyHandle,
};

use crate::managers::{detector_manager::DetectorManager, reaction_manager::ReactionManager};

pub struct Data {
    started_loops: AtomicBool,
    storage_manager: Arc<StorageManager>,
    cotd_manager: Arc<CotdManager>,
    remind_manager: Arc<RemindManager>,
    detector_manager: Arc<DetectorManager>,
    reaction_manager: Arc<ReactionManager>,
    currency_manager: Arc<CurrencyManager>,
    db: Arc<SurrealClient>,
} // User data, which is stored and accessible in all command invocations
pub struct Handler {} // User data, which is stored and accessible in all command invocations
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

pub async fn reply(
    ctx: Context<'_>,
    text: impl Into<String>,
    ephemeral: bool,
) -> Result<crate::ReplyHandle<'_>, crate::serenity::SerenityError> {
    ctx.send(|b| b.content(text).reply(true).ephemeral(ephemeral))
        .await
}

async fn event_handler(
    ctx: &serenity::Context,
    event: &Event<'_>,
    framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        Event::Ready { data_about_bot } => {
            println!("Logged in as {}", data_about_bot.user.tag());
        }
        Event::CacheReady { guilds } => {
            if !data.started_loops.load(Ordering::Relaxed) {
                println!("Caches are ready! Starting all the managers.");
                let arc_ctx = Arc::new(ctx.clone());
                storage_manager_loop(arc_ctx.clone(), data.storage_manager.clone());
                cotd_manager_loop(arc_ctx.clone(), data.db.clone(), data.cotd_manager.clone());
                remind_manager_loop(arc_ctx.clone(), data.remind_manager.clone());
                data.started_loops.swap(true, Ordering::Relaxed);
            }
            // TODO: Look through all relevant data and check if its still valid.
            // If a reminder for a user is in a guild the user is no longer in, remove them.
            // If a reaction role has an emoji, message or role that no longer exists, remove them.
            // If a folder about a guild still exists even though the bot is no longer in the guild, remove them.
            //
        }
        Event::Message { new_message } => {
            //TODO: notify errors to the user/server log
            data.detector_manager.on_message(ctx, new_message).await;
        }
        Event::ReactionAdd { add_reaction } => {
            //TODO: notify errors to the user/server log
            data.reaction_manager
                .reaction_add_event(ctx, add_reaction, framework.bot_id)
                .await;
        }
        Event::ReactionRemoveAll {
            channel_id,
            removed_from_message_id,
        } => {
            //TODO: REmove all reaction roles from the message.
        }
        Event::ReactionRemove { removed_reaction } => {
            //TODO: notify errors to the user/server log
            data.reaction_manager
                .reaction_remove_event(ctx, removed_reaction, framework.bot_id)
                .await;
        }
        _ => {}
    }
    Ok(())
}
#[tokio::main]
async fn main() {
    println!("Starting bot...");

    let db = Arc::new(managers::db::new_db().await);

    let storage_manager = Arc::new(StorageManager::new("./data").await);

    let framework = poise::Framework::builder()
        .token(tokens::get_discord_token())
        .intents(
            serenity::GatewayIntents::GUILDS
                | serenity::GatewayIntents::GUILD_MESSAGES
                | serenity::GatewayIntents::GUILD_MESSAGE_REACTIONS
                | serenity::GatewayIntents::DIRECT_MESSAGES
                | serenity::GatewayIntents::MESSAGE_CONTENT,
        )
        .options(poise::FrameworkOptions {
            commands: commands::get_commands(),
            event_handler: |_ctx: &serenity::Context, event: &Event<'_>, _framework, _data| {
                Box::pin(event_handler(_ctx, event, _framework, _data))
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                println!("Registering commands...");
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                let tokens = tokens::get_other_tokens();
                Ok(Data {
                    cotd_manager: Arc::new(CotdManager::new(db.clone())),
                    remind_manager: Arc::new(RemindManager::new(db.clone())),
                    detector_manager: Arc::new(DetectorManager::new(db.clone())),
                    reaction_manager: Arc::new(ReactionManager::new(db.clone())),
                    currency_manager: Arc::new(
                        CurrencyManager::new(
                            tokens.openexchangerates_token.unwrap_or("".to_string()),
                        )
                        .await,
                    ),
                    storage_manager,
                    started_loops: AtomicBool::new(false),
                    db,
                })
            })
        });

    framework.run().await.unwrap();
}
