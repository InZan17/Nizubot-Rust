use poise::{
    serenity_prelude::{CreateEmbed, CreateEmbedFooter, UserId},
    CreateReply,
};

use crate::{Context, Error};

/// See what order people joined at.
#[poise::command(slash_command, guild_only)]
pub async fn join_order(
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

    let (nearby_members, target_index, stats) = join_order
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
                .footer(CreateEmbedFooter::new(stats))
                .description(description),
        ),
    )
    .await?;

    return Ok(());
}
