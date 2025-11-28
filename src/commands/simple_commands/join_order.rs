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

#[derive(poise::ChoiceParameter, PartialEq)]
pub enum GraphData {
    #[name = "Total members"]
    TotalMembers,
    #[name = "New members"]
    NewMembers,
}

impl GraphData {
    pub fn is_total_members(&self) -> bool {
        match self {
            GraphData::NewMembers => false,
            GraphData::TotalMembers => true,
        }
    }
}

#[derive(poise::ChoiceParameter, PartialEq)]
pub enum GraphType {
    #[name = "Line graph"]
    LineGraph,
    #[name = "Bar graph"]
    BarGraph,
}

/// See a graph of when people joined.
#[poise::command(slash_command)]
pub async fn graph(
    ctx: Context<'_>,
    #[description = "Do you wanna graph members joining or total members?"] graph_data: GraphData,
    #[description = "Do you want a line graph or a bar graph? (New members: Line graph, Total members: Bar graph)"]
    graph_type: Option<GraphType>,
    #[min = 1]
    #[max = 255]
    #[description = "How many entries do you want on the graph? (Default: auto)"]
    entries: Option<usize>,
) -> Result<(), Error> {
    let member_count;
    let guild_id;
    let guild_name;

    {
        let guild = ctx.guild().unwrap();
        member_count = guild.member_count;
        guild_id = guild.id;
        guild_name = guild.name.clone();
    }

    ctx.defer().await?;

    let join_order = ctx.data().join_order_manager.get_join_order(guild_id).await;

    let (sorted_members, comparisons, sort_ms, fetch_ms) = join_order
        .get_sorted_members(member_count, ctx.http())
        .await?;

    let (timezone, time_format) = {
        let profile_data = ctx
            .data()
            .profile_manager
            .get_profile_data(ctx.author().id)
            .await;

        let mut profile_lock = profile_data.lock().await;

        match profile_lock.get_profile(&ctx.data().db).await {
            Ok(profile) => {
                let timezone = profile
                    .get_timezone()
                    .and_then(|(_, tz)| tz)
                    .unwrap_or(Tz::UTC);
                let time_format = profile.get_time_format_with_fallback(ctx.locale().unwrap());
                (timezone, time_format)
            }
            Err(_) => (Tz::UTC, locale_time_format(ctx.locale().unwrap())),
        }
    };

    let start_date = sorted_members
        .iter()
        .find_map(|member| member.joined_at.as_deref().cloned());

    let Some(start_date) = start_date else {
        return Err("Couldn't find start date.".into());
    };
    let start_date = timezone.from_utc_datetime(&start_date.naive_utc());

    let end_date = timezone.from_utc_datetime(&Utc::now().naive_utc());

    let duration = end_date.signed_duration_since(start_date);
    let duration_secs = duration.as_seconds_f64();

    let entries = entries.unwrap_or(
        // Slowly increases the entries the more members there are.
        match graph_data {
            GraphData::NewMembers => {
                ((1.0 - 1.002_f32.powi(-(sorted_members.len() as i32))) * 255.0).ceil() as usize
            }
            GraphData::TotalMembers => {
                ((1.0 - 1.0035_f32.powi(-(sorted_members.len() as i32))) * 255.0).ceil() as usize
            }
        }
        .clamp(14, 255),
    );

    let graph_type = graph_type.unwrap_or_else(|| match graph_data {
        GraphData::NewMembers => GraphType::BarGraph,
        GraphData::TotalMembers => GraphType::LineGraph,
    });

    let duration_between = duration / entries as i32;

    let mut amount_during = vec![0_usize; entries + 1];

    for member in sorted_members.iter() {
        let Some(joined_at) = member.joined_at else {
            continue;
        };

        for i in 0..(entries + 1) {
            let before_date = start_date + duration_between * (i as i32);
            if *joined_at < before_date {
                amount_during[i] += 1;
                break;
            }
        }
    }

    let members_in_graph: usize = amount_during.iter().sum();

    if graph_data.is_total_members() {
        let mut seen = 0;
        for amount in amount_during.iter_mut() {
            seen += *amount;
            *amount = seen;
        }
    }

    let max_value = amount_during.iter().max().copied().unwrap_or(1);

    let width = 1280;

    let height = 720;

    let now = get_current_ms_time();

    let start_date_string = {
        let time = get_time_string(start_date, time_format);
        format!(
            "{}-{}-{} {}",
            start_date.year(),
            start_date.month(),
            start_date.day(),
            time
        )
    };

    let end_date_timezone = timezone.from_utc_datetime(&end_date.naive_utc());

    let end_date_string = {
        let time = get_time_string(end_date_timezone, time_format);
        format!(
            "{}-{}-{} {}",
            end_date_timezone.year(),
            end_date_timezone.month(),
            end_date_timezone.day(),
            time
        )
    };

    let mut buffer = vec![0; RGBPixel::PIXEL_SIZE * (width * height) as usize];
    {
        let drawing_area =
            BitMapBackend::with_buffer(&mut buffer, (width, height)).into_drawing_area();

        drawing_area.fill(&WHITE)?;
        let title = format!("{} graph for {guild_name}", graph_data.name());

        drawing_area.titled(&title, ("arial", 0.08 * height as f32))?;

        let mut chart = ChartBuilder::on(&drawing_area)
            .x_label_area_size(7.percent_height())
            .y_label_area_size(7.percent_height())
            .margin_left(2.percent_height())
            .margin_right(7.percent_height())
            .margin_top(9.percent_height())
            .margin_bottom(2.percent_height())
            .caption(
                format!(
                    "{start_date_string}  =>  {end_date_string}  ({})",
                    timezone.name()
                ),
                ("arial", 5.percent_height()),
            )
            .build_cartesian_2d(0..entries, 0..max_value)?;

        chart
            .configure_mesh()
            .x_label_style(("arial", 2.6.percent_height()))
            .y_label_style(("arial", 3.percent_height()))
            .y_desc(graph_data.name())
            .x_labels(14)
            .y_labels(10)
            .x_label_formatter(&|i| {
                let time_mult = (entries as f64 / 14.0).min(1.0);
                let current_date = start_date + duration_between * (*i as i32);

                if duration_secs > 60000000. * time_mult {
                    // More than almost 2 years.
                    // Show year and month.
                    current_date.format("%Y-%b").to_string()
                } else if duration_secs > 86400. * 14. * time_mult {
                    //More than 14 days.
                    // Show month and day.
                    current_date.format("%b-%d").to_string()
                } else {
                    // Less than 14 days.
                    // Show day name and time.
                    let time_string = get_time_string(current_date, time_format);
                    current_date
                        .format(&format!("%a {time_string}"))
                        .to_string()
                }
            })
            .draw()?;

        match graph_type {
            GraphType::LineGraph => {
                chart.draw_series(LineSeries::new(
                    amount_during.iter().enumerate().map(|(x, y)| (x, *y)),
                    RED.mix(0.5).filled().stroke_width((height / 240).max(2)),
                ))?;
            }
            GraphType::BarGraph => {
                chart.draw_series(
                    Histogram::vertical(&chart)
                        .style(RED.mix(0.5).filled())
                        .data(
                            amount_during
                                .iter()
                                .enumerate()
                                .map(|(x, y)| (x.saturating_sub(1), if x == 0 { 0 } else { *y })),
                        ),
                )?;
            }
        }

        let (_, lower) = drawing_area.split_vertically(height - height / 22);

        lower.titled(
            &format!("Total guild members: {member_count} | Members in graph: {members_in_graph} | Total entries: {}", match graph_type {
                GraphType::LineGraph => entries + 1,
                GraphType::BarGraph => entries,
            }),
            ("arial", 0.026 * height as f32).into_font().color(&BLACK.mix(0.75)),
        )?;
    }

    let image = RgbImage::from_vec(width, height, buffer).ok_or("Failed to create RgbImage")?;

    let mut png_bytes = Vec::new();

    let png_encoder = PngEncoder::new(&mut png_bytes);

    image.write_with_encoder(png_encoder)?;

    let draw_ms = get_current_ms_time() - now;

    ctx.send(
        CreateReply::default()
            .attachment(CreateAttachment::bytes(png_bytes, "graph.png")).embed(
            CreateEmbed::new()
                .title(format!("{} graph for {guild_name}", graph_data.name()))
                .description(format!("
                    From **{start_date_string}** to **{end_date_string}** ({})

                    *(NOTE: This graph only includes users that are currently in the server.)*
                ", timezone.name()))
                .attachment("graph.png")
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
