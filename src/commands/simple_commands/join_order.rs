use chrono::{Datelike, TimeZone, Utc};
use chrono_tz::Tz;
use image::{codecs::png::PngEncoder, RgbImage};
use plotters::{
    backend::{PixelFormat, RGBPixel},
    chart::ChartBuilder,
    prelude::{BitMapBackend, IntoDrawingArea},
    series::{Histogram, LineSeries},
    style::{AsRelative, Color, IntoFont, BLACK, RED, WHITE},
};
use poise::{
    serenity_prelude::{CreateAttachment, CreateEmbed, CreateEmbedFooter, UserId},
    ChoiceParameter, CreateReply,
};

use crate::{
    commands::{
        simple_commands::ping::get_current_ms_time,
        utility_commands::check_timezone::get_time_string,
    },
    managers::profile_manager::locale_time_format,
    Context, Error,
};

pub mod graph;

use graph::graph;

#[poise::command(
    slash_command,
    guild_only,
    subcommands("get", "graph"),
    subcommand_required
)]
pub async fn join_order(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// See what order people joined at.
#[poise::command(slash_command)]
pub async fn get(
    ctx: Context<'_>,
    #[description = "Which user do you wanna check?"] user: Option<UserId>,
    #[description = "Which which index you wanna check?"] index: Option<usize>,
) -> Result<(), Error> {
    if user.is_some() && index.is_some() {
        ctx.send(
            CreateReply::default()
                .content("Please do not use the 'user' and 'index' options at the same time.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }
    let member_count;
    let guild_id;

    {
        let guild = ctx.guild().unwrap();
        member_count = guild.member_count;
        guild_id = guild.id;
    }

    if member_count > 10000 {
        ctx.send(
            CreateReply::default()
                .content("Too many members!")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    ctx.defer().await?;

    let target_user_id = user.unwrap_or(ctx.author().id);

    let mut description = "".to_owned();

    let author_id = ctx.author().id;

    let join_order = ctx.data().join_order_manager.get_join_order(guild_id).await;

    let (nearby_members, target_index, comparisons, sort_ms, fetch_ms) = join_order
        .get_members_around_user_or_index(member_count, target_user_id, index, ctx.http())
        .await?;

    for (i, member) in nearby_members {
        description.push_str(format!("**{i}.** {}", member.tag).as_str());

        if member.id == author_id {
            description.push_str(" ***(you)***\n");
        } else if target_index == i || target_user_id == member.id {
            description.push_str(" ***(target)***\n");
        } else {
            description.push_str("\n");
        }
    }

    ctx.send(
        CreateReply::default().embed(
            CreateEmbed::new()
                .title("Join order")
                .footer(CreateEmbedFooter::new(
                    if let Some(fetch_ms) = fetch_ms {
                        format!(
                            "sorting took {comparisons} comparisons and {sort_ms}ms. Fetching members took {fetch_ms}ms."
                        )
                    } else {
                        format!("sorting took {comparisons} comparisons and {sort_ms}ms.")
                    }
                ))
                .description(description),
        ),
    )
    .await?;

    return Ok(());
}
