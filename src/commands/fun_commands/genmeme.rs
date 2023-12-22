use core::slice::SlicePattern;

use std::{
    ops::Deref,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use poise::serenity_prelude::{AttachmentType, User};
use rand::{random, Rng};
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::{managers::storage_manager, Context, Error};

/// I will generate a meme.
#[poise::command(slash_command, subcommands("brick"))]
pub async fn genmeme(ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

const BRICK_TITLES: [&str; 6] = [
    "<user> kindly gives you a brick.",
    "<user> throws a brick at you.",
    "<user> shares their brick with you.",
    "This brick is a gift from <user>.",
    "\"Think fast!\" -<user>",
    "Look, up in the sky! It's a bird! It's a plane! It's <user>'s brick!"
];

/// Generate a gif of some user throwing a brick.
#[poise::command(slash_command)]
pub async fn brick(
    // TODO: better error messages
    ctx: Context<'_>,
    #[description = "The user to throw the brick."] user: Option<User>,
) -> Result<(), Error> {
    let storage_manager = &ctx.data().storage_manager;

    let user = user.unwrap_or(ctx.author().clone());

    let avatar_url = user.avatar_url().unwrap_or(user.default_avatar_url());

    let saved_normal_pfp = storage_manager
        .get_data_or_default(
            vec!["users", &user.id.to_string(), "saved_normal_pfp"],
            "".to_string(),
        )
        .await;

    let saved_brick_pfp = storage_manager
        .get_data_or_default(
            vec!["users", &user.id.to_string(), "saved_brick_pfp"],
            "".to_string(),
        )
        .await;

    let user_folder = format!("{}/users/{}", storage_manager.storage_path, user.id);

    let generated_folder = format!("{user_folder}/generated");

    let brick_gif_file = format!("{generated_folder}/brick.gif");
    let user_pfp_file = format!("{user_folder}/pfp.png"); //File extension can be wrong. Doesn't matter tho since ffmpeg will pick up the right one afterwards hopefully.

    let brick_gif = "generate_materials/brick.gif".to_string();

    fs::create_dir_all(generated_folder).await?; //once generated folder is created, user folder is also created.

    if saved_normal_pfp.get_data().await.deref() != &avatar_url
        || !Path::new(&user_pfp_file).exists()
    {
        let resp = reqwest::get(&avatar_url).await?;
        if !resp.status().is_success() {
            return Err(Error::from(resp.text().await?));
        }

        let avatar_bytes = resp.bytes().await?;
        let mut avatar_file = fs::File::create(&user_pfp_file).await?;
        avatar_file.write_all(avatar_bytes.as_slice()).await?;
        *saved_normal_pfp.get_data_mut().await = avatar_url.clone();
        saved_normal_pfp.request_file_write().await;
    }

    if saved_brick_pfp.get_data().await.deref() != &avatar_url
        || !Path::new(&brick_gif_file).exists()
    {
        let mut process = tokio::process::Command::new("ffmpeg");

        process.args(&["-i", &brick_gif]);
        process.args(&["-i", &user_pfp_file]);
        process.args(&["-ss", "00:00:00"]);
        process.args(&["-t", "00:00:02"]);
        process.args(&["-filter_complex", "[1:v]format=rgba,scale=48:48,pad=width=300:height=300:x=114:y=252:color=0x00000000[ico];[ico][0:v]overlay=0:0:enable='between(t,0,20)',split=2[out1][out2];[out2]palettegen=reserve_transparent=on[p];[out1][p]paletteuse"]);
        process.arg(&brick_gif_file);
        process.arg("-y");

        let mut spawned = process.spawn()?;
        let exit = spawned.wait().await?;

        if !exit.success() {
            let mut stderr = spawned.stderr.take();
            if let Some(stderr) = &mut stderr {
                let mut dst = String::new();
                let _ = stderr.read_to_string(&mut dst).await;

                ctx.send(|m| m.content(format!("Sorry, I couldn't generate the gif.\n\n{dst}")))
                    .await?;
                return Ok(());
            }

            ctx.send(|m| m.content("Sorry, I couldn't generate the gif."))
                .await?;
            return Ok(());
        }

        *saved_brick_pfp.get_data_mut().await = avatar_url.clone();
        saved_brick_pfp.request_file_write().await;
    }

    let brick_file = fs::File::open(brick_gif_file).await?;

    

    ctx.send(|m| {
        m.attachment(AttachmentType::File { file: &brick_file, filename: "brick.gif".to_string()})
        .embed(|e| {
            e.footer(|f| {
                f.text("Original gif by \"mega-KOT\" on newgrounds.\nhttps://www.newgrounds.com/art/view/mega-kot/think-fast")
            })
            .attachment("brick.gif");
        
            let mut rng = rand::thread_rng();
            let random_index = rng.gen_range(0..BRICK_TITLES.len());
            let random_title = BRICK_TITLES[random_index].replace("<user>", &user.name);

            e.title(random_title)
        })
    }).await?;

    Ok(())
}