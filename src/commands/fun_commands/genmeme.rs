use crate::{Context, Error};
use core::slice::SlicePattern;
use evalexpr::context_map;
use poise::{
    serenity_prelude::{Attachment, AttachmentType, User},
    SlashChoiceParameter,
};
use rand::Rng;
use tokio::{fs, io::AsyncWriteExt};

mod brick;

/// I will generate a meme.
#[poise::command(slash_command, subcommands("brick", "caption"))]
pub async fn genmeme(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

const BRICK_TITLES: [&str; 6] = [
    "<user> kindly gives you a brick.",
    "<user> throws a brick at you.",
    "<user> shares their brick with you.",
    "This brick is a gift from <user>.",
    "\"Think fast!\" -<user>",
    "Look, up in the sky! It's a bird! It's a plane! It's <user>'s brick!",
];

/// Generate a gif of some user throwing a brick.
#[poise::command(slash_command)]
pub async fn brick(
    // TODO: better error messages
    // TODO: Make a cooldown per user
    ctx: Context<'_>,
    #[description = "The user to throw the brick."] user: Option<User>,
) -> Result<(), Error> {
    let storage_manager = &ctx.data().storage_manager;

    let user = user.unwrap_or(ctx.author().clone());

    let brick_gif_file = brick::gen_brick_gif(storage_manager, &user).await?;

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

#[derive(poise::ChoiceParameter, PartialEq)]
enum CaptionType {
    #[name = "White boxes"]
    Boxes,
    #[name = "WHAT"]
    What,
    #[name = "Overlay text"]
    Overlay,
}

impl CaptionType {
    fn to_string(&self) -> String {
        match self {
            CaptionType::Boxes => "boxes".to_owned(),
            CaptionType::What => "what".to_owned(),
            CaptionType::Overlay => "overlay".to_owned(),
        }
    }
}

/// Generate an image with captions.
#[poise::command(slash_command)]
pub async fn caption(
    ctx: Context<'_>,
    #[description = "The image to be captioned."] image: Attachment,
    #[description = "Which type of caption you want."] caption_type: CaptionType,
    #[description = "What the upper text should be. (type \"\\n\" to make a new line.)"] upper_text: Option<String>,
    #[description = "What the bottom text should be. (type \"\\n\" to make a new line.)"]
    bottom_text: Option<String>,
    #[description = "Size of the font. (WHAT: width/7, Boxes: width/10, Overlay: height/10)"]
    font_size: Option<String>,
    #[description = "How big the space between new lines should be. (Default: fontsize/4)"]
    break_height: Option<String>,
    #[description = "Amount of empty space around the text. (WHAT: width/9, Boxes: width/20, Overlay: height/30)"]
    padding: Option<String>,
) -> Result<(), Error> {
    const TWELVE_MIB_IN_BYTES: u64 = 12582912;

    if image.size > TWELVE_MIB_IN_BYTES {
        ctx.send(|m| m.content("Please make sure your image is 12 MiB or less in size."))
            .await?;
        return Ok(());
    }

    if upper_text.is_none() && bottom_text.is_none() {
        ctx.send(|m| m.content("Please provide some text.")).await?;
        return Ok(());
    }

    let content_type = image.content_type.unwrap_or(String::new());

    let content_type_vec = content_type.split("/").collect::<Vec<&str>>();

    if content_type_vec.len() != 2 {
        ctx.send(|m| {
            m.content("Sorry, I couldn't make sense of the files content type. Please make sure your file isn't corrupted.")
        }).await?;
        return Ok(());
    }

    if content_type_vec[0] != "image" && content_type_vec[0] != "video" {
        ctx.send(|m| m.content("Please provide an actual image or video."))
            .await?;
        return Ok(());
    }

    if image.width.is_none() || image.height.is_none() {
        ctx.send(|m| {
            m.content("Sorry, I couldn't get the width and/or height of the image. Please make sure your file isn't corrupted.")
        }).await?;
        return Ok(());
    }

    let width = image.width.unwrap_or(0);
    let height = image.height.unwrap_or(0);

    let mut context = context_map! {
        "width" => evalexpr::Value::Int(width as i64),
        "height" => evalexpr::Value::Int(height as i64),
    }
    .unwrap();

    let font_size_expression = font_size.unwrap_or_else(|| match caption_type {
        CaptionType::Boxes => "width/10".to_owned(),
        CaptionType::What => "width/7".to_owned(),
        CaptionType::Overlay => "height/10".to_owned(),
    });
    //TODO: If an error occurs, also provide the expression which was faulty for easier debugging.
    let font_size = evalexpr::eval_number_with_context(&font_size_expression, &context)?;

    if font_size < 0. {
        ctx.send(|m| m.content("Please make sure \"font_size\" isn't a negative number."))
            .await?;
        return Ok(());
    }

    evalexpr::eval_empty_with_context_mut(&format!("fontsize = {font_size}"), &mut context)?;

    let break_height_expression = break_height.unwrap_or_else(|| "fontsize/4".to_owned());
    let break_height = evalexpr::eval_number_with_context(&break_height_expression, &context)?;

    if break_height < 0. {
        ctx.send(|m| m.content("Please make sure \"break_height\" isn't a negative number."))
            .await?;
        return Ok(());
    }

    let padding_expression = padding.unwrap_or_else(|| match caption_type {
        CaptionType::Boxes => "width/20".to_owned(),
        CaptionType::What => "width/9".to_owned(),
        CaptionType::Overlay => "height/30".to_owned(),
    });
    let padding = evalexpr::eval_number_with_context(&padding_expression, &context)?;

    if padding < 0. {
        ctx.send(|m| m.content("Please make sure \"padding\" isn't a negative number."))
            .await?;
        return Ok(());
    }

    let extension = content_type_vec[1];

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

    let storage_manager = &ctx.data().storage_manager;

    let caption_folder = format!("{}/generated/caption", storage_manager.storage_path);
    let images_folder = format!("{}/downloads/images", storage_manager.storage_path);

    fs::create_dir_all(&caption_folder).await?;
    fs::create_dir_all(&images_folder).await?;

    let generated_file = format!("{caption_folder}/{}.{extension}", ctx.id());
    let downloaded_file = format!("{images_folder}/{}.{extension}", ctx.id());

    let mut ffmpeg_filter = "[0]".to_string();

    let font = {
        if caption_type == CaptionType::What {
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

    let mut spawned = process.spawn()?;
    let exit = spawned.wait().await?;

    if !exit.success() {
        return Err(Error::from(format!("`ffmpeg` exited with {}", exit)));
    }

    let generated_image_file = fs::File::open(generated_file).await?;

    ctx.send(|m| {
        m.attachment(AttachmentType::File {
            file: &generated_image_file,
            filename: format!(
                "{}_{}.{}",
                image.filename,
                caption_type.to_string(),
                extension
            ),
        })
    })
    .await?;

    Ok(())
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
