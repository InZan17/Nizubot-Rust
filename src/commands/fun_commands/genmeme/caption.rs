use crate::Error;
use core::slice::SlicePattern;
use evalexpr::context_map;
use poise::serenity_prelude::Attachment;
use tokio::{fs, io::AsyncWriteExt};

use super::CaptionType;

pub async fn caption(
    id: u64,
    storage_path: &String,
    image: &Attachment,
    caption_type: &CaptionType,
    upper_text: Option<String>,
    bottom_text: Option<String>,
    font_size_expr: Option<String>,
    break_height_expr: Option<String>,
    padding_expr: Option<String>,
    extension: &String,
) -> Result<String, Error> {
    let width = image.width.unwrap_or(0);
    let height = image.height.unwrap_or(0);

    let mut context = context_map! {
        "width" => evalexpr::Value::Int(width as i64),
        "height" => evalexpr::Value::Int(height as i64),
    }
    .unwrap();

    let font_size_expr = font_size_expr.unwrap_or_else(|| match caption_type {
        CaptionType::Boxes => "width/10".to_owned(),
        CaptionType::What => "width/7".to_owned(),
        CaptionType::Overlay => "height/10".to_owned(),
    });

    let font_size = match evalexpr::eval_number_with_context(&font_size_expr, &context) {
        Ok(ok) => ok,
        Err(err) => {
            return Err(
                format!("Couldn't evaluate font_size with \"{font_size_expr}\". {err}").into(),
            )
        }
    };

    if font_size < 0. {
        return Err(Error::from("`font_size` cannot be a negative number."));
    }

    evalexpr::eval_empty_with_context_mut(&format!("fontsize = {font_size}"), &mut context)?;

    let break_height_expr = break_height_expr.unwrap_or_else(|| "fontsize/4".to_owned());
    let break_height = match evalexpr::eval_number_with_context(&break_height_expr, &context) {
        Ok(ok) => ok,
        Err(err) => {
            return Err(format!(
                "Couldn't evaluate break_height with \"{break_height_expr}\". {err}"
            )
            .into())
        }
    };

    if break_height < 0. {
        return Err(Error::from("`break_height` cannot be a negative number."));
    }

    evalexpr::eval_empty_with_context_mut(&format!("break_height = {break_height}"), &mut context)?;

    let padding_expr = padding_expr.unwrap_or_else(|| match caption_type {
        CaptionType::Boxes => "width/20".to_owned(),
        CaptionType::What => "width/9".to_owned(),
        CaptionType::Overlay => "height/30".to_owned(),
    });
    let padding = match evalexpr::eval_number_with_context(&padding_expr, &context) {
        Ok(ok) => ok,
        Err(err) => {
            return Err(format!("Couldn't evaluate padding with \"{padding_expr}\". {err}").into())
        }
    };

    if padding < 0. {
        return Err(Error::from("`padding` cannot be a negative number."));
    }

    let upper_texts = {
        if let Some(upper_text) = &upper_text {
            upper_text.split("\\n").collect::<Vec<&str>>()
        } else {
            vec![]
        }
    };

    let bottom_texts = {
        if let Some(bottom_text) = &bottom_text {
            bottom_text.split("\\n").collect::<Vec<&str>>()
        } else {
            vec![]
        }
    };

    let caption_folder = format!("{}/generated/caption", storage_path);
    let images_folder = format!("{}/downloads/images", storage_path);

    fs::create_dir_all(&caption_folder).await?;
    fs::create_dir_all(&images_folder).await?;

    let generated_file = format!("{caption_folder}/{}.{extension}", id);
    let downloaded_file = format!("{images_folder}/{}.{extension}", id);

    let mut ffmpeg_filter = "[0]".to_string();

    let font = {
        if *caption_type == CaptionType::What {
            "Times New Roman"
        } else {
            "Impact"
        }
    };

    let font_color = {
        match caption_type {
            CaptionType::Boxes => "black".to_owned(),
            CaptionType::What => "white".to_owned(),
            CaptionType::Overlay => format!("white:bordercolor=black:borderw={}", font_size / 20.),
        }
    };

    match caption_type {
        CaptionType::Boxes => {
            if bottom_texts.len() > 0 {
                let bottom_height = padding * 2.
                    + font_size * (bottom_texts.len() as f64)
                    + break_height * (bottom_texts.len() as f64 - 1.);
                ffmpeg_filter = format!(
                    "{ffmpeg_filter}pad=width=iw:height=ih+{bottom_height}:x=0:y=0:color=0xFFFFFF,"
                )
            }

            if upper_texts.len() > 0 {
                let upper_height = padding * 2.
                    + font_size * (upper_texts.len() as f64)
                    + break_height * (upper_texts.len() as f64 - 1.);
                ffmpeg_filter = format!("{ffmpeg_filter}pad=width=iw:height=ih+{upper_height}:y={upper_height}:color=0xFFFFFF,")
            }
        }
        CaptionType::What => {
            let small_border_size = (height as f64 / 297.).ceil() * 2.;
            let big_border_size = (height as f64 / 74.).ceil() * 2.;

            let bottom_height = padding
                + font_size * bottom_texts.len() as f64
                + break_height * (bottom_texts.len() as f64 - 1.).max(0.);
            let upper_height = padding
                + font_size * upper_texts.len() as f64
                + break_height * (upper_texts.len() as f64 - 1.).max(0.);

            ffmpeg_filter = format!("{ffmpeg_filter}pad=width=iw+{small_border_size}:height=ih+{small_border_size}:x=iw/2:y=ih/2:color=0x000000,");
            ffmpeg_filter = format!("{ffmpeg_filter}pad=width=iw+{big_border_size}:height=ih+{big_border_size}:x=iw/2:y=ih/2:color=0xFFFFFF,");

            ffmpeg_filter = format!(
                "{ffmpeg_filter}pad=width=iw:height=ih+{}:y={upper_height}:color=0x000000,",
                upper_height + bottom_height
            );

            ffmpeg_filter = format!(
                "{ffmpeg_filter}pad=width=ih*({width}/{height}):x=(iw-out_w)/2:color=0x000000,"
            );
        }
        CaptionType::Overlay => {}
    }

    let font_ascent = font_size * (1638. / 2048.);
    let font_descent = font_size - font_ascent;

    for (index, text) in upper_texts.into_iter().enumerate() {
        let alignment_offset = format!("-max_glyph_a+{}", font_ascent + font_descent / 2.);
        let line_offset = padding + (font_size + break_height) * index as f64;

        let sanitized_text = sanitize_string(text);

        ffmpeg_filter = format!("{ffmpeg_filter}drawtext=text='{sanitized_text}':font={font}:x=(main_w-text_w)/2:y={alignment_offset}+{line_offset}:fontsize={font_size}:fontcolor={font_color},");
    }

    let bottom_texts_len = bottom_texts.len();

    for (index, text) in bottom_texts.into_iter().enumerate() {
        let alignment_offset = format!(
            "-max_glyph_a-{}",
            font_size - font_ascent - font_descent / 2.
        );
        let line_offset =
            padding + (font_size + break_height) * (bottom_texts_len - 1 - index) as f64;

        let sanitized_text = sanitize_string(text);

        ffmpeg_filter = format!("{ffmpeg_filter}drawtext=text='{sanitized_text}':font={font}:x=(main_w-text_w)/2:y=main_h{alignment_offset}-{line_offset}:fontsize={font_size}:fontcolor={font_color},");
    }

    if extension == "gif" {
        ffmpeg_filter = format!("{ffmpeg_filter}split=2[s0][s1];[s0]palettegen=reserve_transparent=on[p];[s1][p]paletteuse,")
    }

    let resp = reqwest::get(&image.url).await?;
    if !resp.status().is_success() {
        return Err(Error::from(resp.text().await?));
    }

    let image_bytes = resp.bytes().await?;
    let mut image_file = fs::File::create(&downloaded_file).await?;
    image_file.write_all(image_bytes.as_slice()).await?;

    let mut ffmpeg_filter_chars = ffmpeg_filter.chars();

    ffmpeg_filter_chars.next_back(); // remove the last character because it will just be a ','

    ffmpeg_filter = ffmpeg_filter_chars.as_str().to_string();

    println!("{ffmpeg_filter}");

    let mut process = tokio::process::Command::new("ffmpeg");

    process.args(&["-i", &downloaded_file]);
    process.args(&["-filter_complex", &ffmpeg_filter]);
    process.arg(&generated_file);
    process.arg("-y");

    //TODO: Make ffmpeg not write to disk and make everything be in ram.

    let mut spawned = process.spawn()?;
    let exit = spawned.wait().await?;

    if !exit.success() {
        return Err(Error::from(format!("`ffmpeg` exited with {}", exit)));
    }

    return Ok(generated_file);
}

fn sanitize_string(text: &str) -> String {
    text.replace("\\", "\\\\")
        .replace("%%", "%%%%")
        .replace(":", "\\:")
        .replace(";", "\\;")
        .replace("|", "\\|")
        .replace("<", "\\<")
        .replace(">", "\\>")
        .replace("{", "\\{")
        .replace("}", "\\}")
        .replace("\"", "\\\"")
        .replace("'", "''")
}
