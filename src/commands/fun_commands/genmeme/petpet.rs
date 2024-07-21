use core::slice::SlicePattern;
use std::{ops::Deref, path::Path, sync::Arc};

use poise::serenity_prelude::User;
use tokio::{fs, io::AsyncWriteExt};

use crate::{
    managers::storage_manager::{DataType, StorageManager},
    Error,
};

pub async fn gen_petpet_gif(
    storage_manager: &Arc<StorageManager>,
    user: &User,
) -> Result<String, Error> {
    let avatar_url = user.avatar_url().unwrap_or(user.default_avatar_url());

    let saved_user_pfp_file = format!("saved_{}_pfp.txt", user.id);
    let saved_user_petpet_file = format!("saved_{}_petpet.txt", user.id);

    let saved_normal_pfp = storage_manager
        .load_disk_or(&saved_user_pfp_file, true, DataType::String("".to_string()))
        .await?
        .string()
        .unwrap()
        .clone();
    let saved_petpet_pfp = storage_manager
        .load_disk_or(
            &saved_user_petpet_file,
            true,
            DataType::String("".to_string()),
        )
        .await?
        .string()
        .unwrap()
        .clone();

    let petpet_gif_file = format!("{}_petpet.gif", user.id);
    let user_pfp_file = format!("{}_pfp.png", user.id); //File extension can be wrong. Doesn't matter tho since ffmpeg will pick up the right one afterwards hopefully.

    let petpet_gif = "generate_materials/petpet.gif".to_string();

    if *saved_normal_pfp != avatar_url || !Path::new(&user_pfp_file).exists() {
        let resp = reqwest::get(&avatar_url).await?;
        if !resp.status().is_success() {
            return Err(Error::from(resp.text().await?));
        }

        let avatar_bytes = resp.bytes().await?;
        storage_manager
            .save_disk(
                &user_pfp_file,
                &DataType::Bytes(avatar_bytes.as_slice().to_vec()),
            )
            .await?;

        storage_manager
            .save_disk(&saved_user_pfp_file, &DataType::String(avatar_url.clone()))
            .await?;
    }

    println!(
        "saved_pfp_is NOT _saved {}",
        *saved_petpet_pfp != avatar_url
    );
    println!("there is no file {}", !Path::new(&petpet_gif_file).exists());

    if *saved_petpet_pfp != avatar_url || !Path::new(&petpet_gif_file).exists() {
        let mut process = tokio::process::Command::new("ffmpeg");

        const HEIGHT_SQUISH: &str = "round((sin(t*10*PI-PI*0.4)+0.75)*8)";
        const SIDE_SQUISH: &str = "round((sin(t*10*PI-PI*0.4)+0.75)*4)";
        const SIDE_OFFSET: &str = "round((sin(t*10*PI-PI*0.5)+0.9)*2)";

        process.args(&["-i", &petpet_gif]);
        process.args(&["-loop", "1"]);
        process.args(&["-i", &storage_manager.get_full_directory(&user_pfp_file)]);
        process.args(&["-f", "lavfi"]);
        process.args(&["-i", "color=size=112x112:color=#00000000,format=rgba"]);
        process.args(&[
            "-filter_complex",
            &format!(
                "[1:v]format=rgba,
                scale=
                83+{SIDE_SQUISH}:
                83-{HEIGHT_SQUISH}:
                eval=frame[ico];
                [2:v][ico]
                overlay=
                112-85-(overlay_w-83)-{SIDE_OFFSET}:
                112-83-(overlay_h-83)+1:
                eval=frame[ico];
                [ico][0:v]overlay=0:0:shortest=1,split=2[out1][out2];
                [out2]palettegen=reserve_transparent=on[p];
                [out1][p]paletteuse"
            ),
        ]);
        process.arg(storage_manager.get_full_directory(&petpet_gif_file));
        process.arg("-y");

        let mut spawned = process.spawn()?;
        let exit = spawned.wait().await?;

        if !exit.success() {
            return Err(Error::from(format!("`ffmpeg` exited with {}", exit)));
        }

        storage_manager
            .save_disk(
                &saved_user_petpet_file,
                &DataType::String(avatar_url.clone()),
            )
            .await?;
    }

    Ok(petpet_gif_file)
}
