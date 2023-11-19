use crate::{managers::cotd_manager::SECONDS_IN_A_DAY, Context, Error};
use poise::serenity_prelude::{AttachmentType, Timestamp};

/// Get the current color of the day.
#[poise::command(slash_command)]
pub async fn cotd(
    ctx: Context<'_>,
    #[description = "The day you wanna get the color of."] day: Option<u64>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let cotd_manager = &ctx.data().cotd_manager;
    let color_info;

    let working_day;

    let title;
    let date_description;
    let date_offset;

    if let Some(day) = day {
        match cotd_manager.get_day_color(day).await {
            Ok(color) => {
                color_info = color;
                working_day = day;
                title = format!("Color of day {working_day}");
                date_description = "COTD during";
                date_offset = 0;
            }
            Err(err) => {
                ctx.reply(err).await?;
                return Ok(());
            }
        }
    } else {
        match cotd_manager.get_current_color().await {
            Ok(color) => {
                color_info = color;
                working_day = cotd_manager.get_current_day();
                title = "Color of the day".to_owned();
                date_description = "Next color";
                date_offset = 86400;
            }
            Err(err) => {
                ctx.reply(err).await?;
                return Ok(());
            }
        }
    }

    let png_bytes = create_color_png(&color_info.hex);

    let file_name = format!("{}.png", color_info.hex);

    ctx.send(|m| {
        m.attachment(AttachmentType::Bytes {
            data: std::borrow::Cow::Borrowed(png_bytes.as_slice()),
            filename: file_name.clone(),
        })
        .embed(|e| {
            e.title(title)
                .description(format!("**{}** (#{})", color_info.name, color_info.hex.to_ascii_uppercase()))
                .image(format!("attachment://{}", file_name))
                .timestamp(
                    Timestamp::from_unix_timestamp(
                        (working_day * SECONDS_IN_A_DAY + date_offset) as i64,
                    )
                    .unwrap(),
                )
                .footer(|f| f.text(format!("Day {working_day} | {date_description}")))
        })
    })
    .await?;
    Ok(())
}

pub fn create_color_png(hex: &String) -> Vec<u8> {
    const PLTE_CRC_HASH: u32 = 1269336405;

    let color_value = u32::from_str_radix(&hex, 16).unwrap();

    let mut crc = crc32fast::Hasher::new_with_initial(PLTE_CRC_HASH);
    crc.update(&[
        ((color_value >> 16) & 255) as u8,
        ((color_value >> 8) & 255) as u8,
        (color_value & 255) as u8,
    ]);
    let color_hash_result = crc.finalize();

    vec![
        0x89,
        0x50,
        0x4E,
        0x47,
        0x0D,
        0x0A,
        0x1A,
        0x0A,
        0x00,
        0x00,
        0x00,
        0x0D,
        0x49,
        0x48,
        0x44,
        0x52,
        0x00,
        0x00,
        0x00,
        0xFF,
        0x00,
        0x00,
        0x00,
        0xFF,
        0x01,
        0x03,
        0x00,
        0x00,
        0x00,
        0x04,
        0xC6,
        0x92,
        0x89,
        0x00,
        0x00,
        0x00,
        0x03,
        0x50, //P
        0x4C, //L
        0x54, //T
        0x45, //E
        ((color_value >> 16) & 255) as u8,
        ((color_value >> 8) & 255) as u8,
        (color_value & 255) as u8,
        ((color_hash_result >> 24) & 255) as u8,
        ((color_hash_result >> 16) & 255) as u8,
        ((color_hash_result >> 8) & 255) as u8,
        (color_hash_result & 255) as u8,
        0x00,
        0x00,
        0x00,
        0x23,
        0x49,
        0x44,
        0x41,
        0x54,
        0x78,
        0xDA,
        0xEC,
        0xC0,
        0x31,
        0x01,
        0x00,
        0x00,
        0x00,
        0xC2,
        0x20,
        0xFB,
        0xA7,
        0xB6,
        0xC6,
        0x0E,
        0x18,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x40,
        0xDB,
        0x03,
        0x00,
        0x00,
        0xFF,
        0xFF,
        0x20,
        0xDF,
        0x00,
        0x01,
        0xCB,
        0x33,
        0x38,
        0x4F,
        0x00,
        0x00,
        0x00,
        0x00,
        0x49,
        0x45,
        0x4E,
        0x44,
        0xAE,
        0x42,
        0x60,
        0x82,
    ]
}
