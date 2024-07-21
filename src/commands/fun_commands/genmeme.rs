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
mod caption;
mod petpet;

/// I will generate a meme.
#[poise::command(slash_command, subcommands("brick", "petpet", "caption"))]
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
    ctx: Context<'_>,
    #[description = "The user to throw the brick."] user: Option<User>,
) -> Result<(), Error> {
    let storage_manager = &ctx.data().storage_manager;

    let user = user.unwrap_or(ctx.author().clone());

    ctx.defer().await?;

    let brick_gif_file = brick::gen_brick_gif(storage_manager, &user).await?;

    let brick_file = fs::File::open(storage_manager.get_full_directory(&brick_gif_file)).await?;

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

const PETPET_TITLES: [&str; 3] = ["PETTHE<USER>", "rt to pet <user>", "mmmm myes pet <user>"];

/// Generate a gif of some user getting petted.
#[poise::command(slash_command)]
pub async fn petpet(
    ctx: Context<'_>,
    #[description = "The user to be petted."] user: Option<User>,
) -> Result<(), Error> {
    let storage_manager = &ctx.data().storage_manager;

    let user = user.unwrap_or(ctx.author().clone());

    ctx.defer().await?;

    let petpet_gif_file = petpet::gen_petpet_gif(storage_manager, &user).await?;

    let petpet_file = fs::File::open(storage_manager.get_full_directory(&petpet_gif_file)).await?;

    ctx.send(|m| {
        m.attachment(AttachmentType::File {
            file: &petpet_file,
            filename: "petpet.gif".to_string(),
        })
        .embed(|e| {
            e.footer(|f| f.text("Original hand video from DitzyFlama on twitter.\nhttps://x.com/DitzyFlama/status/1229852204082679809"))
                .attachment("petpet.gif");

            let mut rng = rand::thread_rng();
            let random_index = rng.gen_range(0..PETPET_TITLES.len());
            let random_title = PETPET_TITLES[random_index]
                .replace("<user>", &user.name)
                .replace("<USER>", &user.name.to_uppercase());

            e.title(random_title)
        })
    })
    .await?;

    Ok(())
}

#[derive(poise::ChoiceParameter, PartialEq)]
pub enum CaptionType {
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
        ctx.send(|m| {
            m.content("Please make sure your image is 12 MiB or less in size.")
                .ephemeral(true)
        })
        .await?;
        return Ok(());
    }

    if upper_text.is_none() && bottom_text.is_none() {
        ctx.send(|m| m.content("Please provide some text.").ephemeral(true))
            .await?;
        return Ok(());
    }

    let content_type = image.content_type.clone().unwrap_or(String::new());

    let content_type_vec = content_type.split("/").collect::<Vec<&str>>();

    if content_type_vec.len() != 2 {
        ctx.send(|m| m.content("Sorry, I couldn't make sense of the files content type. Please make sure your file isn't corrupted.").ephemeral(true)).await?;
        return Ok(());
    }

    if content_type_vec[0] != "image" && content_type_vec[0] != "video" {
        ctx.send(|m| {
            m.content("Please provide an actual image or video.")
                .ephemeral(true)
        })
        .await?;
        return Ok(());
    }

    if image.width.is_none() || image.height.is_none() {
        ctx.send(|m| m.content("Sorry, I couldn't get the width and/or height of the image. Please make sure your file isn't corrupted.").ephemeral(true)).await?;
        return Ok(());
    }

    let extension = content_type_vec[1].to_owned();

    ctx.defer().await?;

    let generated_file_path = caption::caption(
        ctx.id(),
        &ctx.data().storage_manager.storage_path_string,
        &image,
        &caption_type,
        upper_text,
        bottom_text,
        font_size,
        break_height,
        padding,
        &extension,
    )
    .await?;

    let generated_image_file = fs::File::open(generated_file_path).await?;

    ctx.send(|m| {
        m.attachment(AttachmentType::File {
            file: &generated_image_file,
            filename: format!(
                "{}_{}.{}",
                remove_extension(&image.filename),
                caption_type.to_string(),
                extension
            ),
        })
    })
    .await?;

    Ok(())
}

fn remove_extension(file_name: &str) -> String {
    let mut parts: Vec<&str> = file_name.split('.').collect();

    if parts.len() > 1 {
        parts.pop();
    }

    parts.join(".")
}
