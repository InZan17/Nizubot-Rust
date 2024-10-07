use std::time::{SystemTime, UNIX_EPOCH};

use poise::{
    serenity_prelude::{CreateEmbed, CreateEmbedFooter, Member, UserId},
    CreateReply,
};

use crate::{Context, Error};

/// See what order people joined at.
#[poise::command(slash_command, guild_only)]
pub async fn joinorder(
    ctx: Context<'_>,
    #[description = "Which user you wanna check."] user: Option<UserId>,
    #[description = "Which index you wanna check."] index: Option<usize>,
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
    let mut members: Vec<Member>;
    let member_count;
    let guild_id;

    {
        let guild = ctx.guild().unwrap();
        members = guild.members.values().cloned().collect();
        member_count = guild.member_count;
        guild_id = guild.id;
    }

    if member_count > 1000 {
        ctx.send(
            CreateReply::default()
                .content("Too many members!")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    if (members.len() as u64) < member_count {
        members = guild_id.members(ctx, Some(1000), None).await.unwrap();
    }

    let now = get_current_ms_time();
    let mut comparisons = 0;

    members.sort_unstable_by(|member_a, member_b| {
        let joined_a = member_a.joined_at.unwrap();
        let joined_b = member_b.joined_at.unwrap();
        let cmp = joined_a.partial_cmp(&joined_b).unwrap();
        comparisons += 1;
        cmp
    });

    let difference_ms = get_current_ms_time() - now;

    let mut target_index = 0;

    let target_user_id = user.unwrap_or(ctx.author().id);

    if let Some(index) = index {
        target_index = index as i32;
    } else {
        for (index, member) in members.iter().enumerate() {
            if member.user.id == target_user_id {
                target_index = index as i32;
                break;
            }
        }
    }

    let members_len = members.len() as i32;
    let max_possible_index = members_len - 1;

    let mut max_index = max_possible_index.min(target_index + 4);
    let mut min_index = 0.max(target_index - 4);

    if min_index == 0 {
        max_index = 8.min(max_possible_index);
    } else if max_index == max_possible_index {
        min_index = max_possible_index - 8;
    }

    let mut description = "".to_owned();

    let author_id = ctx.author().id;

    for i in min_index..max_index + 1 {
        let member = &members[i as usize];

        // TODO: Verify that it works correctly when discriminator is None
        let tag = member.user.tag();

        description.push_str(format!("**{i}.** {tag}").as_str());

        if member.user.id == author_id {
            description.push_str(" ***(you)***\n");
        } else if target_index == i || target_user_id == member.user.id {
            description.push_str(" ***(target)***\n");
        } else {
            description.push_str("\n");
        }
    }

    ctx.send(
        CreateReply::default().embed(
            CreateEmbed::new()
                .title("Join order")
                .footer(CreateEmbedFooter::new(format!(
                    "sorting took {comparisons} comparisons and {difference_ms}ms."
                )))
                .description(description),
        ),
    )
    .await?;

    return Ok(());
}

pub fn get_current_ms_time() -> u128 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards. Oopsie.");
    since_the_epoch.as_millis()
}
