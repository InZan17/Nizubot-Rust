use poise::{serenity_prelude::{self as serenity, EventHandler, GuildId, CacheHttp, Ready}, async_trait, Event, ReplyHandle};

use std::sync::atomic::{AtomicU32, Ordering};

mod read;
mod commands;
pub struct Data {} // User data, which is stored and accessible in all command invocations
pub struct Handler {} // User data, which is stored and accessible in all command invocations
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

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
        _ => {}
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    println!("Starting bot...");
    let framework = poise::Framework::builder()
        .token(read::read_token())
        .intents(serenity::GatewayIntents::from_bits_truncate(3243775))

        .options(poise::FrameworkOptions {
            commands: commands::get_commands(),
            event_handler: |_ctx, event, _framework, _data| {
                Box::pin(event_handler(_ctx, event, _framework, _data))
            },
            ..Default::default()
        })

        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                println!("Registering commands...");
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        });

    framework.run().await.unwrap();
}