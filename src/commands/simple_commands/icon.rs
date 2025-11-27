use crate::{Context, Error};
use poise::{
    serenity_prelude::{CreateEmbed, Emoji, User},
    CreateReply,
};

/// Get the icon of whatever you want!
#[poise::command(
    slash_command,
    subcommands("user", "guild", "emoji"),
    subcommand_required,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn icon(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Get the profile picture of a certain user.
#[poise::command(slash_command)]
pub async fn user(
    ctx: Context<'_>,
    #[description = "Which user do you want the pfp from?"] user: Option<User>,
    #[description = "Should the message be hidden from others? (Default: False)"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = ephemeral.unwrap_or(false);
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

    ctx.send(
        CreateReply::default()
            .embed(
                CreateEmbed::new()
                    .title(format!("{name}'s avatar"))
                    .image(avatar_url),
            )
            .ephemeral(ephemeral),
    )
    .await?;
    Ok(())
}

/// Get the icon of the guild.
#[poise::command(slash_command)]
pub async fn guild(
    ctx: Context<'_>,
    #[description = "Should the message be hidden from others? (Default: False)"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = ephemeral.unwrap_or(false);
    let name;
    let icon_url;
    {
        let Some(guild) = &ctx.guild() else {
            ctx.send(
                CreateReply::default()
                    .content("Please run this command in a guild!")
                    .ephemeral(true),
            )
            .await?;
            return Ok(());
        };
        name = guild.name.clone();
        icon_url = guild.icon_url();
    }

    if let Some(icon_url) = icon_url {
        ctx.send(
            CreateReply::default()
                .embed(
                    CreateEmbed::new()
                        .title(format!("{name}'s icon"))
                        .image(format!("{icon_url}?size=1024")),
                )
                .ephemeral(ephemeral),
        )
        .await?;
        return Ok(());
    }

    ctx.send(
        CreateReply::default()
            .content("Sorry, this guild does not have an icon.")
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// Get the icon of a custom emoji.
#[poise::command(slash_command)]
pub async fn emoji(
    ctx: Context<'_>,
    #[description = "Which custom emoji do you want the icon from?"] emoji: Emoji,
    #[description = "Should the message be hidden from others? (Default: False)"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = ephemeral.unwrap_or(false);
    ctx.send(
        CreateReply::default()
            .embed(
                CreateEmbed::new()
                    .title(format!("{}'s icon", emoji.name))
                    .image(format!("{}?size=1024", emoji.url())),
            )
            .ephemeral(ephemeral),
    )
    .await?;
    return Ok(());
}
