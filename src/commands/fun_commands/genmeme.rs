use poise::{serenity_prelude::{AttachmentType, User, Attachment}, SlashChoiceParameter};
use rand::Rng;
use tokio::fs;

use crate::{Context, Error};

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
    "Look, up in the sky! It's a bird! It's a plane! It's <user>'s brick!"
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

#[derive(poise::ChoiceParameter)]
enum CaptionType {
    #[name = "White boxes"]
    Boxes,
    #[name = "WHAT"]
    What,
    #[name = "Overlay text"]
    Overlay
}

/// Generate an image with captions.
#[poise::command(slash_command)]
pub async fn caption(
    ctx: Context<'_>,
    #[description = "The image to be captioned."] image: Attachment,
    #[description = "Which type of caption you want."] caption_type: CaptionType,
    #[description = "What the upper text should be. (type \"\\n\" to make a new line.)"] upper_text: String,
    #[description = "What the bottom text should be. (type \"\\n\" to make a new line.)"] bottom_text: String,
    #[description = "Size of the font. (WHAT: width/7, Boxes: width/10, Overlay: height/10)"] font_size: String,
    #[description = "How big the space between new lines should be. (Default: fontsize/4)"] break_height: String,
    #[description = "Amount of empty space around the text. (WHAT: width/9, Boxes: width/20, Overlay: height/30)"] padding: String,
) -> Result<(), Error> {
    let storage_manager = &ctx.data().storage_manager;

    todo!();
}