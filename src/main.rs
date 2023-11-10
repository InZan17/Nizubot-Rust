#![feature(downcast_unchecked)]
#![feature(get_mut_unchecked)]

mod commands;
pub mod give_up_serialize;
mod managers;
mod read;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use managers::{
    cotd_manager::{CotdManager, cotd_manager_loop},
    storage_manager::{storage_manager_loop, StorageManager},
};
use poise::{serenity_prelude as serenity, Event, ReplyHandle};

pub struct Data {
    started_loops: AtomicBool,
    storage_manager: Arc<StorageManager>,
    cotd_manager: Arc<CotdManager>,
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
        Event::CacheReady { guilds: _ } => {
            if !data.started_loops.load(Ordering::Relaxed) {
                let arc_ctx = Arc::new(ctx.clone());
                storage_manager_loop(arc_ctx.clone(), data.storage_manager.clone());
                cotd_manager_loop(arc_ctx.clone());
                data.started_loops.swap(true, Ordering::Relaxed);
            }
        }
        _ => {}
    }
    Ok(())
}
//1269336405 hash plte
//1370065315 result
#[tokio::main]
async fn main() {
    //temporary code I will use for making an image with a single color
    let mut crc = crc32fast::Hasher::new();
    crc.update(&[0x50, 0x4c, 0x54, 0x45]);
    let plte_result = crc.finalize();
    println!("{}", plte_result);
    let mut crc = crc32fast::Hasher::new_with_initial(plte_result);
    crc.update(&[0xb6, 0xa8, 0x31]);
    let rest_result = crc.finalize();
    println!("{}", rest_result);

    println!("Starting bot...");
    let framework = poise::Framework::builder()
        .token(read::read_token())
        .intents(serenity::GatewayIntents::from_bits_truncate(3243775))
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
                    storage_manager: storage_manager.clone(),
                    cotd_manager: Arc::new(CotdManager::new(storage_manager)),
                    started_loops: AtomicBool::new(false),
                })
            })
        });

    framework.run().await.unwrap();
}
