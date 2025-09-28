use crate::{
    managers::storage_manager::{DataType, StorageManager},
    Context, Error,
};
use poise::{
    serenity_prelude::{
        Attachment, AttachmentId, CreateAttachment, CreateEmbed, CreateEmbedFooter, User, UserId,
    },
    CreateReply,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::fs;
use webp::BitstreamFeatures;

mod brick;
mod caption;
mod petpet;

const BRICK_TITLES: [&str; 6] = [
    "<user> kindly gives you a brick.",
    "<user> throws a brick at you.",
    "<user> shares their brick with you.",
    "This brick is a gift from <user>.",
    "\"Think fast!\" -<user>",
    "Look, up in the sky! It's a bird! It's a plane! It's <user>'s brick!",
];

pub enum ImageIdType {
    User(UserId),
    Attachment(AttachmentId),
}

impl ImageIdType {
    pub fn get(&self) -> u64 {
        match self {
            ImageIdType::User(user_id) => user_id.get(),
            ImageIdType::Attachment(attachment_id) => attachment_id.get(),
        }
    }

    pub fn is_user(&self) -> bool {
        match self {
            ImageIdType::User(_) => true,
            _ => false,
        }
    }
}

pub struct ImageInfo {
    width: u32,
    height: u32,
    input_extension: String,
    output_extension: String,
    download_link: String,
    image_id: ImageIdType,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LastAvatarInfo {
    pub url: String,
    pub resolution: (u32, u32),
}

impl ImageInfo {
    pub fn from_user_or_attachment(
        user: &Option<User>,
        image: &Option<Attachment>,
    ) -> Result<Self, &'static str> {
        match (user, image) {
            (None, None) => Err("Please put something in the `image` field or `user` field."),
            (Some(_), Some(_)) => {
                Err("The `image` field and `user` field cannot be used at the same time.")
            }
            (None, Some(image)) => {
                const SIX_MB_IN_BYTES: u32 = 6000000;

                if image.size > SIX_MB_IN_BYTES {
                    return Err("Please make sure your image is 6 MB or less in size.");
                }

                let content_type = image.content_type.clone().unwrap_or(String::new());

                let content_type_vec = content_type.split("/").collect::<Vec<&str>>();

                if content_type_vec.len() != 2 {
                    return Err("Sorry, I couldn't make sense of the files content type. Please make sure your file isn't corrupted.");
                }

                if content_type_vec[0] != "image" && content_type_vec[0] != "video" {
                    return Err("Please provide an actual image or video.");
                }

                if image.width.is_none() || image.height.is_none() {
                    return Err("Sorry, I couldn't get the width and/or height of the image. Please make sure your file isn't corrupted.");
                }

                Ok(ImageInfo::from_attachment(image))
            }
            (Some(user), None) => Ok(ImageInfo::from_user(user)),
        }
    }

    pub fn from_user(user: &User) -> Self {
        if let (Some(hash), Some(avatar_url)) = (user.avatar, user.avatar_url()) {
            return ImageInfo {
                width: 0,
                height: 0,
                input_extension: if hash.is_animated() {
                    "gif".to_string()
                } else {
                    "webp".to_string()
                },
                output_extension: if hash.is_animated() {
                    "gif".to_string()
                } else {
                    "png".to_string()
                },
                download_link: avatar_url,
                image_id: ImageIdType::User(user.id),
            };
        } else {
            return ImageInfo {
                width: 256,
                height: 256,
                input_extension: "png".to_string(),
                output_extension: "png".to_string(),
                download_link: user.default_avatar_url(),
                image_id: ImageIdType::User(user.id),
            };
        }
    }

    pub fn from_attachment(attachment: &Attachment) -> Self {
        // This might cause some errors later, but the user should've made sure there's an extension. It's their fault.
        let extension = attachment.filename.split(".").last().unwrap_or("png");
        ImageInfo {
            width: attachment.width.unwrap_or(0),
            height: attachment.height.unwrap_or(0),
            input_extension: extension.to_string(),
            output_extension: extension.to_string(),
            download_link: attachment.url.clone(),
            image_id: ImageIdType::Attachment(attachment.id),
        }
    }

    pub async fn download_image(
        &mut self,
        storage_manager: &StorageManager,
    ) -> Result<String, Error> {
        match self.image_id {
            ImageIdType::User(user_id) => {
                let pfps_folder = "downloads/pfps";
                storage_manager.create_dir_all(pfps_folder).await?;

                let saved_user_pfp_file = format!("{pfps_folder}/saved_{}_pfp.json", user_id);

                let saved_normal_pfp = storage_manager
                    .load_disk_or(
                        &saved_user_pfp_file,
                        true,
                        DataType::String("null".to_string()),
                    )
                    .await?
                    .string()
                    .unwrap()
                    .clone();

                let last_avatar_info =
                    serde_json::from_str::<Option<LastAvatarInfo>>(&saved_normal_pfp)?;

                let user_pfp_file =
                    format!("{pfps_folder}/{}_pfp.{}", user_id, self.input_extension);

                let needs_download = if let Some(last_avatar_info) = last_avatar_info {
                    (self.width, self.height) = last_avatar_info.resolution;
                    last_avatar_info.url != self.download_link
                        || !storage_manager.path_exists(&user_pfp_file)
                } else {
                    true
                };

                if needs_download {
                    let resp = reqwest::get(&self.download_link).await?;
                    if !resp.status().is_success() {
                        return Err(Error::from(resp.text().await?));
                    }

                    let avatar_bytes = resp.bytes().await?;

                    let Some(features) = BitstreamFeatures::new(avatar_bytes.as_ref()) else {
                        return Err("Couldn't get image info.".into());
                    };

                    self.width = features.width();
                    self.height = features.height();

                    storage_manager
                        .save_disk(&user_pfp_file, &DataType::Bytes(avatar_bytes.to_vec()))
                        .await?;

                    storage_manager
                        .save_disk(
                            &saved_user_pfp_file,
                            &DataType::String(serde_json::to_string(&LastAvatarInfo {
                                url: self.download_link.clone(),
                                resolution: (features.width(), features.height()),
                            })?),
                        )
                        .await?;
                }
                return Ok(storage_manager.get_full_directory(&user_pfp_file));
            }
            ImageIdType::Attachment(attachment_id) => {
                let images_folder = "downloads/images";
                storage_manager.create_dir_all(images_folder).await?;

                let downloaded_file =
                    format!("{images_folder}/{}.{}", attachment_id, self.input_extension);

                let resp = reqwest::get(&self.download_link).await?;
                if !resp.status().is_success() {
                    return Err(Error::from(resp.text().await?));
                }

                let image_bytes = resp.bytes().await?;
                storage_manager
                    .save_disk(&downloaded_file, &DataType::Bytes(image_bytes.to_vec()))
                    .await?;

                return Ok(storage_manager.get_full_directory(&downloaded_file));
            }
        }
    }
}

/// I will generate a meme.
#[poise::command(
    slash_command,
    subcommands("brick", "petpet", "caption"),
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn genmeme(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Generate a gif of someone throwing a brick.
#[poise::command(slash_command)]
pub async fn brick(
    ctx: Context<'_>,
    #[description = "Which user should throw the brick?"] user: Option<User>,
    #[description = "Which image should throw the brick?"] image: Option<Attachment>,
    #[description = "Should the message be hidden from others?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = ephemeral.unwrap_or(false);

    let image_info = if user.is_none() && image.is_none() {
        ImageInfo::from_user(ctx.author())
    } else {
        match ImageInfo::from_user_or_attachment(&user, &image) {
            Ok(image_info) => image_info,
            Err(err) => {
                ctx.send(CreateReply::default().content(err).ephemeral(true))
                    .await?;
                return Ok(());
            }
        }
    };

    let storage_manager = &ctx.data().storage_manager;

    let user = user.unwrap_or(ctx.author().clone());

    if ephemeral {
        ctx.defer_ephemeral().await?;
    } else {
        ctx.defer().await?;
    }

    let brick_gif_file = brick::gen_brick_gif(storage_manager, image_info).await?;

    let brick_file = fs::File::open(storage_manager.get_full_directory(&brick_gif_file)).await?;

    let random_index;
    let random_title;

    {
        let mut rng = rand::thread_rng();
        random_index = rng.gen_range(0..BRICK_TITLES.len());
        random_title = BRICK_TITLES[random_index].replace("<user>", &user.name);
    }

    ctx.send(CreateReply::default().attachment(CreateAttachment::file ( &brick_file, "brick.gif").await?)
        .embed(CreateEmbed::new().footer(CreateEmbedFooter::new("Original gif by \"mega-KOT\" on newgrounds.\nhttps://www.newgrounds.com/art/view/mega-kot/think-fast"))
            .attachment("brick.gif")

            .title(random_title)
        )).await?;

    Ok(())
}

const PETPET_TITLES: [&str; 3] = ["PETTHE<USER>", "rt to pet <user>", "mmmm myes pet <user>"];

/// Generate a gif of someone getting petted.
#[poise::command(slash_command)]
pub async fn petpet(
    ctx: Context<'_>,
    #[description = "Which user should be petted?"] user: Option<User>,
    #[description = "Which image should be petted?"] image: Option<Attachment>,
    #[description = "Should the message be hidden from others?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = ephemeral.unwrap_or(false);

    let image_info = if user.is_none() && image.is_none() {
        ImageInfo::from_user(ctx.author())
    } else {
        match ImageInfo::from_user_or_attachment(&user, &image) {
            Ok(image_info) => image_info,
            Err(err) => {
                ctx.send(CreateReply::default().content(err).ephemeral(true))
                    .await?;
                return Ok(());
            }
        }
    };

    let storage_manager = &ctx.data().storage_manager;

    let user = user.unwrap_or(ctx.author().clone());

    if ephemeral {
        ctx.defer_ephemeral().await?;
    } else {
        ctx.defer().await?;
    }

    let petpet_gif_file = petpet::gen_petpet_gif(storage_manager, image_info).await?;

    let petpet_file = fs::File::open(storage_manager.get_full_directory(&petpet_gif_file)).await?;

    let random_index;
    let random_title;

    {
        let mut rng = rand::thread_rng();
        random_index = rng.gen_range(0..PETPET_TITLES.len());
        random_title = PETPET_TITLES[random_index]
            .replace("<user>", &user.name)
            .replace("<USER>", &user.name.to_uppercase());
    }

    ctx.send(CreateReply::default().attachment(CreateAttachment::file(
            &petpet_file,
            "petpet.gif".to_string(),
    ).await?)
        .embed(CreateEmbed::new().footer(CreateEmbedFooter::new("Original hand video from DitzyFlama on twitter.\nhttps://x.com/DitzyFlama/status/1229852204082679809"))
                .attachment("petpet.gif")


            .title(random_title)
        )
    )
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
    #[description = "Which type of caption do you want?"] caption_type: CaptionType,
    #[description = "Which user should be captioned?"] user: Option<User>,
    #[description = "Which image should be captioned?"] image: Option<Attachment>,
    #[description = "What should the text at the top be? (type \"\\n\" to make a new line.)"]
    upper_text: Option<String>,
    #[description = "What should the text at the bottom be? (type \"\\n\" to make a new line.)"]
    bottom_text: Option<String>,
    #[description = "What should the size of the font be? (WHAT: width/7, Boxes: width/10, Overlay: height/10)"]
    font_size: Option<String>,
    #[description = "How big should the space between new lines be? (Default: font_size/4)"]
    break_height: Option<String>,
    #[description = "How much empty space should be around the text? (WHAT: width/9, Boxes: width/20, Overlay: height/30)"]
    padding: Option<String>,
    #[description = "Should the message be hidden from others?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = ephemeral.unwrap_or(false);
    let image_info = match ImageInfo::from_user_or_attachment(&user, &image) {
        Ok(image_info) => image_info,
        Err(err) => {
            ctx.send(CreateReply::default().content(err).ephemeral(true))
                .await?;
            return Ok(());
        }
    };

    if upper_text.is_none() && bottom_text.is_none() {
        ctx.send(
            CreateReply::default()
                .content("Please provide some text.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }
    if ephemeral {
        ctx.defer_ephemeral().await?;
    } else {
        ctx.defer().await?;
    }

    let generated_file_path = caption::caption(
        &ctx.data().storage_manager,
        image_info,
        &caption_type,
        upper_text,
        bottom_text,
        font_size,
        break_height,
        padding,
    )
    .await?;

    let generated_image_file = fs::File::open(&generated_file_path).await?;

    let filename = generated_file_path
        .split("/")
        .last()
        .unwrap_or("result.png");

    ctx.send(
        CreateReply::default().attachment(
            CreateAttachment::file(
                &generated_image_file,
                format!("{}_{filename}", caption_type.to_string()),
            )
            .await?,
        ),
    )
    .await?;

    Ok(())
}
