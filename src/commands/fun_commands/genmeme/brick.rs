use core::slice::SlicePattern;
use std::{ops::Deref, path::Path, sync::Arc};

use poise::serenity_prelude::User;
use tokio::{fs, io::AsyncWriteExt};

use crate::{managers::storage_manager::{StorageManager, DataType}, Error};

pub async fn gen_brick_gif(
    storage_manager: &Arc<StorageManager>,
    user: &User,
) -> Result<String, Error> {
    let avatar_url = user.avatar_url().unwrap_or(user.default_avatar_url());

    let saved_user_pfp_file = format!("saved_{}_pfp.txt",user.id);
    let saved_user_brick_file = format!("saved_{}_brick.txt",user.id);

    let saved_normal_pfp = storage_manager.load_disk_or(&saved_user_pfp_file, true, DataType::String("".to_string())).await?.get_string();
    let saved_brick_pfp = storage_manager.load_disk_or(&saved_user_brick_file, true, DataType::String("".to_string())).await?.get_string();

    let brick_gif_file = format!("{}_brick.gif",user.id);
    let user_pfp_file = format!("{}_pfp.png", user.id); //File extension can be wrong. Doesn't matter tho since ffmpeg will pick up the right one afterwards hopefully.

    let brick_gif = "generate_materials/brick.gif".to_string();

    if saved_normal_pfp != avatar_url || !Path::new(&user_pfp_file).exists()
    {
        let resp = reqwest::get(&avatar_url).await?;
        if !resp.status().is_success() {
            return Err(Error::from(resp.text().await?));
        }

        let avatar_bytes = resp.bytes().await?;
        //TODO: do something with error/
        storage_manager.save_disk(
            &user_pfp_file, 
            &DataType::Bytes(avatar_bytes.as_slice().to_vec())
        ).await?;

        storage_manager.save_disk(
            &saved_user_pfp_file, 
            &DataType::String(avatar_url.clone())
        ).await?;
    }

    if saved_brick_pfp != avatar_url || !Path::new(&brick_gif_file).exists() {
        let mut process = tokio::process::Command::new("ffmpeg");

        process.args(&["-i", &brick_gif]);
        process.args(&["-i", &storage_manager.get_full_directory(&user_pfp_file)]);
        process.args(&["-ss", "00:00:00"]);
        process.args(&["-t", "00:00:02"]);
        process.args(&["-filter_complex", "[1:v]format=rgba,scale=48:48,pad=width=300:height=300:x=114:y=252:color=0x00000000[ico];[ico][0:v]overlay=0:0:enable='between(t,0,20)',split=2[out1][out2];[out2]palettegen=reserve_transparent=on[p];[out1][p]paletteuse"]);
        process.arg(storage_manager.get_full_directory(&brick_gif_file));
        process.arg("-y");

        let mut spawned = process.spawn()?;
        let exit = spawned.wait().await?;

        if !exit.success() {
            return Err(Error::from(format!("`ffmpeg` exited with {}", exit)));
        }

        storage_manager.save_disk(
            &saved_user_brick_file,
            &DataType::String(avatar_url.clone())
        ).await?;
    }

    Ok(brick_gif_file)
}
