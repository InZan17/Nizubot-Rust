use crate::{Context, Error};
use poise::serenity_prelude::{Emoji, User};

/// Get the icon of whatever you want!
#[poise::command(
    slash_command,
    subcommands("user", "guild", "emoji"),
    subcommand_required
)]
pub async fn icon(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Get the profile picture of a certain user.
#[poise::command(slash_command)]
pub async fn user(
    ctx: Context<'_>,
    #[description = "The user to get the profile picture from."] user: Option<User>,
) -> Result<(), Error> {
    let target_user;
    if let Some(user) = user.as_ref() {
        target_user = user;
    } else {
        target_user = ctx.author();
    }

    let name = &target_user.name;
    let avatar_url = target_user
        .avatar_url()
        .unwrap_or(target_user.default_avatar_url());

    ctx.send(|m| m.embed(|embed| embed.title(format!("{name}'s avatar")).image(avatar_url)))
        .await?;
    Ok(())
}

/// Get the icon of the guild.
#[poise::command(slash_command)]
pub async fn guild(ctx: Context<'_>) -> Result<(), Error> {
    if let Some(guild) = &ctx.guild() {
        let name = &guild.name;
        if let Some(icon_url) = guild.icon_url() {
            ctx.send(|m| {
                m.embed(|embed| {
                    embed
                        .title(format!("{name}'s icon"))
                        .image(format!("{icon_url}?size=1024"))
                })
            })
            .await?;
            return Ok(());
        }

        ctx.send(|m| {
            m.content("Sorry, this guild does not have an icon.")
                .ephemeral(true)
        })
        .await?;
        return Ok(());
    }

    ctx.send(|m| {
        m.content("Please run this command in a guild!")
            .ephemeral(true)
    })
    .await?;
    Ok(())
}

/// Get the icon of a custom emoji.
#[poise::command(slash_command)]
pub async fn emoji(
    ctx: Context<'_>,
    #[description = "The custom emoji to get the icon from."] emoji: Emoji,
) -> Result<(), Error> {
    ctx.send(|m| {
        m.embed(|embed| {
            embed
                .title(format!("{}'s icon", emoji.name))
                .image(format!("{}?size=1024", emoji.url()))
        })
    })
    .await?;
    return Ok(());
}
