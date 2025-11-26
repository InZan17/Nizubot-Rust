use std::sync::Arc;

use crate::{
    managers::storage_manager::{DataType, StorageManager},
    Error,
};

use super::ImageInfo;

pub async fn gen_brick_gif(
    storage_manager: &Arc<StorageManager>,
    mut image_info: ImageInfo,
) -> Result<String, Error> {
    let image_path = image_info.download_image(storage_manager).await?;

    let file_name_id = format!(
        "{}{}",
        if image_info.image_id.is_user() {
            "pfp"
        } else {
            "image"
        },
        image_info.image_id.get()
    );

    let generate;
    let brick_gif_file = format!("{}_brick.gif", file_name_id);
    let saved_user_brick_file = format!("saved_{}_brick.txt", file_name_id);

    if image_info.image_id.is_user() {
        let saved_brick_file = storage_manager
            .load_disk_or(
                &saved_user_brick_file,
                true,
                DataType::String("".to_string()),
            )
            .await?
            .string()
            .unwrap()
            .to_string();

        generate = saved_brick_file != image_info.download_link
            || !storage_manager.path_exists(&brick_gif_file);
    } else {
        generate = true;
    }

    if generate {
        let brick_gif = "generate_materials/brick.gif";

        let mut process = tokio::process::Command::new("ffmpeg");

        process.args(&["-i", brick_gif]);
        process.args(&["-i", &image_path]);
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

        if image_info.image_id.is_user() {
            storage_manager
                .save_disk(
                    &saved_user_brick_file,
                    &DataType::String(image_info.download_link),
                )
                .await?;
        }
    }

    Ok(brick_gif_file)
}
