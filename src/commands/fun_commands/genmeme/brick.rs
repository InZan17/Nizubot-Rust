use std::{sync::Arc, ops::Deref, path::Path};
use core::slice::SlicePattern;

use poise::serenity_prelude::User;
use tokio::{fs, io::AsyncWriteExt};

use crate::{managers::storage_manager::StorageManager, Error};




pub async fn gen_brick_gif(storage_manager: &Arc<StorageManager>, user: &User) -> Result<String, Error>{
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
            return Err(Error::from(format!("`ffmpeg` exited with {}",exit)))
        }

        *saved_brick_pfp.get_data_mut().await = avatar_url.clone();
        saved_brick_pfp.request_file_write().await;
    }

    Ok(brick_gif_file)
}