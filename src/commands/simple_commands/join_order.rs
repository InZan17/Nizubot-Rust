use chrono::Utc;
use image::{codecs::png::PngEncoder, RgbImage};
use plotters::{
    backend::{PixelFormat, RGBPixel},
    chart::ChartBuilder,
    prelude::{BitMapBackend, IntoDrawingArea},
    series::Histogram,
    style::{Color, RED, WHITE},
};
use poise::{
    serenity_prelude::{CreateAttachment, CreateEmbed, CreateEmbedFooter, UserId},
    CreateReply,
};

use crate::{commands::simple_commands::ping::get_current_ms_time, Context, Error};

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

/// See a graph of when people joined.
#[poise::command(slash_command)]
pub async fn graph(ctx: Context<'_>) -> Result<(), Error> {
    let member_count;
    let guild_id;
    let guild_name;

    {
        let guild = ctx.guild().unwrap();
        member_count = guild.member_count;
        guild_id = guild.id;
        guild_name = guild.name.clone();
    }

    let join_order = ctx.data().join_order_manager.get_join_order(guild_id).await;

    let (sorted_members, comparisons, sort_ms, fetch_ms) = join_order
        .get_sorted_members(member_count, ctx.http())
        .await?;

    let start_date = sorted_members
        .iter()
        .find_map(|member| member.joined_at.as_deref().cloned());

    let Some(start_date) = start_date else {
        return Err("Couldn't find start date.".into());
    };

    let end_date = Utc::now();

    let bars = 14_usize;

    let duration = end_date.signed_duration_since(start_date);
    let duration_secs = duration.as_seconds_f64();

    let duration_between = duration / bars as i32;

    let mut amount_joined_during = vec![0_usize; bars];

    for member in sorted_members.iter() {
        let Some(joined_at) = member.joined_at else {
            continue;
        };

        for i in 0..bars {
            let before_date = start_date + duration_between * (i as i32 + 1);
            if *joined_at <= before_date {
                amount_joined_during[i] += 1;
                break;
            }
        }
    }

    let max_value = amount_joined_during.iter().max().copied().unwrap_or(1);

    let width = 850;

    let height = 480;

    let now = get_current_ms_time();

    let start_date_string = start_date.format("%Y-%b-%d %R");
    let end_date_string = end_date.format("%Y-%b-%d %R");

    let mut buffer = vec![0; RGBPixel::PIXEL_SIZE * (width * height) as usize];
    {
        let drawing_area =
            BitMapBackend::with_buffer(&mut buffer, (width, height)).into_drawing_area();

        drawing_area.fill(&WHITE)?;
        drawing_area.titled("Join graph", ("arial", 50))?;

        let mut chart = ChartBuilder::on(&drawing_area)
            .x_label_area_size(35)
            .y_label_area_size(35)
            .margin_left(10)
            .margin_right(35)
            .margin_top(45)
            .margin_bottom(10)
            .caption(
                format!("{start_date_string} => {end_date_string}",),
                ("arial", 25.0),
            )
            .build_cartesian_2d(0..bars, 0..max_value)?;

        chart
            .configure_mesh()
            .label_style(("arial", 12.0))
            .x_labels(14)
            .y_labels(10)
            .x_label_formatter(&|i| {
                let current_date = start_date + duration_between * (*i as i32);
                if duration_secs > 60000000. {
                    // More than almost 2 years.
                    // Show year and month.
                    current_date.format("%Y-%b").to_string()
                } else if duration_secs > 86400. * bars as f64 {
                    //More than (bars) days.
                    // Show month and day.
                    current_date.format("%b-%d").to_string()
                } else {
                    // Less than (bars) days.
                    // Show day name and time.
                    current_date.format("%a %R").to_string()
                }
            })
            .draw()?;

        chart.draw_series(
            Histogram::vertical(&chart)
                .style(RED.mix(0.5).filled())
                .data(
                    amount_joined_during
                        .iter()
                        .enumerate()
                        .map(|(x, y)| (x, *y)),
                ),
        )?;
    }

    let image = RgbImage::from_vec(width, height, buffer).ok_or("Failed to create RgbImage")?;

    let mut png_bytes = Vec::new();

    let png_encoder = PngEncoder::new(&mut png_bytes);

    image.write_with_encoder(png_encoder)?;

    let draw_ms = get_current_ms_time() - now;

    ctx.send(
        CreateReply::default()
            .attachment(CreateAttachment::bytes(png_bytes, "join_graph.png")).embed(
            CreateEmbed::new()
                .title(format!("Join graph for {guild_name}"))
                .description(format!("From **{start_date_string}** to **{end_date_string}**"))
                .attachment("join_graph.png")
                .footer(CreateEmbedFooter::new(
                    if let Some(fetch_ms) = fetch_ms {
                        format!("Drawing took {draw_ms}ms. Sorting took {comparisons} comparisons and {sort_ms}ms. Fetching members took {fetch_ms}ms.")
                    } else {
                        format!("Drawing took {draw_ms}ms. Sorting took {comparisons} comparisons and {sort_ms}ms.")
                    }
                )),
        ),
    )
    .await?;

    return Ok(());
}
