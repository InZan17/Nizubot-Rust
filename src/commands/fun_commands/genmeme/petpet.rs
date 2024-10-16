use std::sync::Arc;

use crate::{
    managers::storage_manager::{DataType, StorageManager},
    Error,
};

use super::ImageInfo;

pub async fn gen_petpet_gif(
    storage_manager: &Arc<StorageManager>,
    mut image_info: ImageInfo,
) -> Result<String, Error> {
    let image_path = image_info.download_image(&storage_manager).await?;

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
    let petpet_gif_file = format!("{}_petpet.gif", file_name_id);
    let saved_user_petpet_file = format!("saved_{}_petpet.txt", file_name_id);

    if image_info.image_id.is_user() {
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

        generate = saved_petpet_pfp != image_info.download_link
            || !storage_manager.path_exists(&petpet_gif_file);
    } else {
        generate = true;
    }

    if generate {
        let petpet_gif = "generate_materials/petpet.gif";

        let mut process = tokio::process::Command::new("ffmpeg");

        const HEIGHT_SQUISH: &str = "round((sin(t*10*PI-PI*0.4)+0.75)*8)";
        const SIDE_SQUISH: &str = "round((sin(t*10*PI-PI*0.4)+0.75)*4)";
        const SIDE_OFFSET: &str = "round((sin(t*10*PI-PI*0.5)+0.9)*2)";

        process.args(&["-i", &petpet_gif]);
        process.args(&["-stream_loop", "-1"]);
        process.args(&["-i", &image_path]);
        process.args(&["-f", "lavfi"]);
        process.args(&["-i", "color=size=112x112:color=#00000000,format=rgba"]);
        process.args(&[
            "-filter_complex",
            &format!(
                "[1:v]format=rgba,
                fps=50,
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

        if image_info.image_id.is_user() {
            storage_manager
                .save_disk(
                    &saved_user_petpet_file,
                    &DataType::String(image_info.download_link),
                )
                .await?;
        }
    }

    Ok(petpet_gif_file)
}
