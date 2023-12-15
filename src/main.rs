#![feature(downcast_unchecked)]
#![feature(get_mut_unchecked)]

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
    remind_manager::{remind_manager_loop, RemindManager},
    storage_manager::{storage_manager_loop, StorageManager},
};
use poise::{serenity_prelude as serenity, Event, ReplyHandle};
use tokens::Tokens;

use crate::managers::detector_manager::DetectorManager;

pub struct Data {
    started_loops: AtomicBool,
    storage_manager: Arc<StorageManager>,
    cotd_manager: Arc<CotdManager>,
    remind_manager: Arc<RemindManager>,
    detector_manager: Arc<DetectorManager>,
    tokens: Tokens,
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
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        Event::Ready { data_about_bot } => {
            println!("Logged in as {}", data_about_bot.user.tag());
        }
        Event::CacheReady { guilds } => {
            if !data.started_loops.load(Ordering::Relaxed) {
                let arc_ctx = Arc::new(ctx.clone());
                storage_manager_loop(arc_ctx.clone(), data.storage_manager.clone());
                cotd_manager_loop(
                    arc_ctx.clone(),
                    data.storage_manager.clone(),
                    data.cotd_manager.clone(),
                );
                remind_manager_loop(arc_ctx.clone(), data.remind_manager.clone());
                data.started_loops.swap(true, Ordering::Relaxed);
            }
        }
        Event::Message { new_message } => {
            data.detector_manager.on_message(ctx, new_message).await;
        }
        _ => {}
    }
    Ok(())
}
#[tokio::main]
async fn main() {
    println!("Starting bot...");
    let framework = poise::Framework::builder()
        .token(tokens::get_discord_token())
        .intents(
            serenity::GatewayIntents::GUILD_MESSAGES
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
                let storage_manager = Arc::new(StorageManager::new("./data").await);
                Ok(Data {
                    cotd_manager: Arc::new(CotdManager::new(storage_manager.clone())),
                    remind_manager: Arc::new(RemindManager::new(storage_manager.clone())),
                    detector_manager: Arc::new(DetectorManager::new(storage_manager.clone())),
                    storage_manager,
                    started_loops: AtomicBool::new(false),
                    tokens: tokens::get_other_tokens(),
                })
            })
        });

    framework.run().await.unwrap();
}
