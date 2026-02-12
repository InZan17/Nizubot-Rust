use std::{collections::HashMap, str::FromStr};

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use poise::{
    serenity_prelude::{
        self, CreateEmbed, CreateEmbedFooter, EmojiId, MessageId, ReactionType, Role, RoleId,
    },
    CreateReply,
};

async fn autocomplete_message_id(
    ctx: Context<'_>,
    partial: &str,
) -> Vec<poise::serenity_prelude::AutocompleteChoice> {
    let Some(guild_id) = ctx.guild_id() else {
        return vec![];
    };

    let messages_data = ctx
        .data()
        .reaction_manager
        .get_messages_data(guild_id)
        .await;

    let mut locked_messages_data = messages_data.lock().await;

    let Ok(messages) = locked_messages_data.get_messages(&ctx.data().db).await else {
        return vec![];
    };

    let channel_id = ctx.channel_id();

    messages
        .iter()
        .filter(|(k, v)| {
            (v.channel_id.is_none() || v.channel_id == Some(channel_id))
                && k.to_string().starts_with(partial)
        })
        .map(|(k, _)| serenity_prelude::AutocompleteChoice::new(k.to_string(), k.to_string()))
        .collect::<Vec<_>>()
}

async fn autocomplete_message_id_unrestricted(
    ctx: Context<'_>,
    partial: &str,
) -> Vec<poise::serenity_prelude::AutocompleteChoice> {
    let Some(guild_id) = ctx.guild_id() else {
        return vec![];
    };

    let messages_data = ctx
        .data()
        .reaction_manager
        .get_messages_data(guild_id)
        .await;

    let mut locked_messages_data = messages_data.lock().await;

    let Ok(messages) = locked_messages_data.get_messages(&ctx.data().db).await else {
        return vec![];
    };

    messages
        .iter()
        .filter(|(k, _)| k.to_string().starts_with(partial))
        .map(|(k, _)| serenity_prelude::AutocompleteChoice::new(k.to_string(), k.to_string()))
        .collect::<Vec<_>>()
}

async fn autocomplete_emoji(
    ctx: Context<'_>,
    partial: &str,
) -> Vec<poise::serenity_prelude::AutocompleteChoice> {
    let Some(guild_id) = ctx.guild_id() else {
        return vec![];
    };

    let poise::Context::Application(application_context) = ctx else {
        return vec![];
    };

    let Some(serenity_prelude::ResolvedValue::String(message_id_string)) = application_context
        .args
        .iter()
        .find(|arg| arg.name == "message_id")
        .map(|arg| arg.value.clone())
    else {
        return vec![];
    };

    let Ok(message_id) = MessageId::from_str(message_id_string) else {
        return vec![];
    };

    let messages_data = ctx
        .data()
        .reaction_manager
        .get_messages_data(guild_id)
        .await;

    let mut locked_messages_data = messages_data.lock().await;

    let Ok(messages) = locked_messages_data.get_messages(&ctx.data().db).await else {
        return vec![];
    };

    let Some(message) = messages.get(&message_id) else {
        return vec![];
    };

    let channel_id = ctx.channel_id();

    if message.channel_id.is_some() && message.channel_id != Some(channel_id) {
        return vec![];
    }

    let guild_emojis = if let Some(guild) = ctx.guild() {
        guild
            .emojis
            .iter()
            .map(|(k, v)| (*k, v.name.clone()))
            .collect()
    } else {
        HashMap::new()
    };

    let matcher = SkimMatcherV2::default().ignore_case();

    let mut emojis = message
        .reaction_roles
        .iter()
        .map(|(k, _)| {
            let is_custom_emoji = k.chars().all(char::is_numeric);

            if is_custom_emoji {
                let mut final_emoji_name = "custom";

                if let Ok(emoji_id) = EmojiId::from_str(k) {
                    if let Some(emoji_name) = guild_emojis.get(&emoji_id) {
                        final_emoji_name = emoji_name;
                    }
                }
                format!("<:{final_emoji_name}:{k}>")
            } else {
                k.clone()
            }
        })
        .filter(|key| matcher.fuzzy_match(key, partial).is_some())
        .collect::<Vec<_>>();

    emojis.sort_by_key(|key| matcher.fuzzy_match(key, partial).unwrap_or(-1));

    emojis
        .into_iter()
        .map(|key| serenity_prelude::AutocompleteChoice::new(key.clone(), key))
        .collect()
}

use crate::{managers::reaction_manager::ReactionTypeOrRoleId, Context, Error};

/// Manage reactions so you get roles when clicking them.
#[poise::command(
    slash_command,
    subcommands("add", "remove", "list"),
    subcommand_required,
    guild_only,
    install_context = "Guild",
    interaction_context = "Guild",
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn reaction_role(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Add reaction role to message.
#[poise::command(
    slash_command,
    required_bot_permissions = "VIEW_CHANNEL | READ_MESSAGE_HISTORY | MANAGE_ROLES | ADD_REACTIONS"
)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "Which message do you wanna add a reaction role to?"] message_id: MessageId,
    #[description = "Which emoji do you want me to react with?"] emoji: ReactionType,
    #[description = "Which role do you want me to give?"] role: Role,
) -> Result<(), Error> {
    let message = ctx.channel_id().message(ctx, message_id).await.map_err(|err| format!("Failed to get the message. Have you tried running this command in the channel the message is located?\n\n{err}"))?;

    let cloned_guild = {
        let Some(guild) = ctx.guild() else {
            return Err("Not in a guild.".into());
        };

        guild.clone()
    };

    let bot_id = ctx.serenity_context().cache.current_user().id;

    let bot_member = cloned_guild
        .member(ctx, bot_id)
        .await
        .map_err(|err| format!("Bot member (self) not found. {err}"))?;

    let highest_role_position = bot_member
        .roles
        .iter()
        .filter_map(|role_id| Some(cloned_guild.roles.get(role_id)?.position as i32))
        .max()
        .unwrap_or(-1);

    if role.position as i32 >= highest_role_position {
        ctx.send(CreateReply::default().content("Sorry, this role is not lower than my highest role and I wont be able to assign it to anyone.").ephemeral(true)
        ).await?;
        return Ok(());
    }

    if let Err(err) = message.react(ctx, emoji.clone()).await {
        ctx.send(CreateReply::default().content(format!("Sorry, I couldn't react with the emoji you provided. Please make sure to provide an actual emoji.\n\nHere's the error: {}", err)).ephemeral(true)
        ).await?;
        return Ok(());
    }

    let guild_id = cloned_guild.id;

    let res = ctx
        .data()
        .reaction_manager
        .add_reaction(emoji, role.id, guild_id, message.channel_id, message.id)
        .await;

    if let Err(err) = res {
        ctx.send(
            CreateReply::default()
                .content(format!(
                    "Sorry, I wasn't able to add that reaction role.\n\n{}",
                    err.to_string()
                ))
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    ctx.send(CreateReply::default().content(format!("Successfully added reaction role!\nTo remove the reaction role, simply remove my reaction or run `/reaction_role remove`.")).ephemeral(true)
    ).await?;

    Ok(())
}

/// Remove reaction role from message.
#[poise::command(
    slash_command,
    required_bot_permissions = "VIEW_CHANNEL | READ_MESSAGE_HISTORY"
)]
pub async fn remove(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_message_id"]
    #[description = "Which message do you wanna remove a reaction role from?"]
    message_id: MessageId,
    #[autocomplete = "autocomplete_emoji"]
    #[description = "What emoji does the reaction role use?"]
    emoji: Option<ReactionType>,
    #[description = "What role does the reaction role use?"] role: Option<RoleId>,
    #[description = "Should I remove all reaction roles from the message?"] remove_all: Option<
        bool,
    >,
) -> Result<(), Error> {
    let message = ctx.channel_id().message(ctx, message_id).await.map_err(|err| format!("Failed to get the message. Have you tried running this command in the channel the message is located?\n\n{err}"))?;

    if role.is_some() as u8 + emoji.is_some() as u8 + remove_all.is_some() as u8 > 1 {
        ctx.send(
            CreateReply::default()
                .content("Please make sure only one of the `emoji`, `role` and `remove_all` parameters are used.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let emoji_or_role;
    if let Some(role) = role {
        emoji_or_role = Some(ReactionTypeOrRoleId::RoleId(role))
    } else if let Some(emoji) = emoji.clone() {
        emoji_or_role = Some(ReactionTypeOrRoleId::ReactionType(emoji))
    } else if let Some(remove_all) = remove_all {
        if !remove_all {
            ctx.send(
                CreateReply::default()
                    .content(
                        "When using the `remove_all` parameter, make sure to set the value to `True` to confirm you wanna remove all reaction roles.",
                    )
                    .ephemeral(true),
            )
            .await?;
            return Ok(());
        }
        emoji_or_role = None
    } else {
        ctx.send(
            CreateReply::default()
                .content("Please make sure only one of the `emoji` and `role` parameters are used.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let guild_id = ctx.guild_id().unwrap();
    let message_id = message.id;

    if let Some(emoji_or_role) = emoji_or_role {
        let removed_role_res = ctx
            .data()
            .reaction_manager
            .remove_reaction_role(emoji_or_role, guild_id, message_id)
            .await;

        match removed_role_res {
            Ok(removed_role) => {
                if let Some(emoji) = emoji {
                    let _ = message
                        .delete_reaction(ctx, Some(ctx.framework().bot_id), emoji)
                        .await;
                }

                ctx.send(
                    CreateReply::default()
                        .content(format!(
                            "Successfully removed reaction role! <@&{}>",
                            removed_role
                        ))
                        .ephemeral(true),
                )
                .await?;
            }
            Err(err) => {
                ctx.send(
                    CreateReply::default()
                        .content(format!(
                            "Sorry, I wasn't able to remove that reaction role.\n\n{}",
                            err.to_string()
                        ))
                        .ephemeral(true),
                )
                .await?;
            }
        }
    } else {
        if let Err(err) = ctx
            .data()
            .reaction_manager
            .remove_all_reaction_roles(guild_id, message_id)
            .await
        {
            ctx.send(
                CreateReply::default()
                    .content(format!(
                        "Sorry, I wasn't able to remove the reaction roles.\n\n{}",
                        err.to_string()
                    ))
                    .ephemeral(true),
            )
            .await?;
        } else {
            ctx.send(
                CreateReply::default()
                    .content("Successfully removed all reaction roles!")
                    .ephemeral(true),
            )
            .await?;
        }
    };

    Ok(())
}

/// List all reaction roles in this guild or for a message.
#[poise::command(slash_command)]
pub async fn list(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_message_id_unrestricted"]
    #[description = "Which message do you wanna check reaction role for?"]
    message_id: Option<MessageId>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    if let Some(message_id) = message_id {
        let reaction_roles = ctx
            .data()
            .reaction_manager
            .get_reaction_roles(guild_id, message_id)
            .await;

        match reaction_roles {
            Err(err) => {
                ctx.send(
                    CreateReply::default()
                        .content(format!(
                            "Sorry, I wasn't able to list the message's reaction roles.\n\n{}",
                            err.to_string()
                        ))
                        .ephemeral(true),
                )
                .await?;
                return Ok(());
            }
            Ok(reaction_roles) => {
                let mut keys = reaction_roles.keys().collect::<Vec<_>>();

                keys.sort();

                let keys_len = keys.len();

                let mut description = String::new();
                for key in keys {
                    let role_id = reaction_roles.get(key).unwrap();

                    let is_custom_emoji = key.chars().all(char::is_numeric);

                    let emoji;
                    let raw_emoji;

                    if is_custom_emoji {
                        raw_emoji = key.clone();
                        emoji = format!("<:custom:{key}>");
                    } else {
                        raw_emoji = format!("\\{key}");
                        emoji = key.clone();
                    }

                    description = format!("{description}{emoji} ({raw_emoji}): <@&{role_id}>\n");
                }

                ctx.send(
                    CreateReply::default()
                        .embed(
                            CreateEmbed::new()
                                .title("Reaction Roles")
                                .description("All of the reaction roles for this message.")
                                .footer(CreateEmbedFooter::new(format!(
                                    "Total reactors: {keys_len}"
                                )))
                                .description(description),
                        )
                        .ephemeral(true),
                )
                .await?;
            }
        }
    } else {
        let reaction_messages = ctx
            .data()
            .reaction_manager
            .get_reaction_role_messages(guild_id)
            .await;

        match reaction_messages {
            Err(err) => {
                ctx.send(
                    CreateReply::default()
                        .content(format!(
                        "Sorry, I wasn't able to list reaction role messages in this guild.\n\n{}",
                        err.to_string()
                    ))
                        .ephemeral(true),
                )
                .await?;
                return Ok(());
            }
            Ok(reaction_messages) => {
                let mut description = String::new();
                for (message_id, channel_id, reaction_count) in reaction_messages.iter() {
                    let append = if let Some(channel_id) = channel_id {
                        format!("[{message_id}](https://discord.com/channels/{guild_id}/{channel_id}/{message_id})")
                    } else {
                        message_id.to_string()
                    };
                    description = format!(
                        "{description}{append}: {reaction_count} reaction role{}.\n",
                        if *reaction_count == 1 { "" } else { "s" }
                    );
                }

                ctx.send(
                    CreateReply::default()
                        .embed(
                            CreateEmbed::new()
                                .title("Reaction Roles Messages")
                                .description("All of the reaction role messages for this guild.")
                                .footer(CreateEmbedFooter::new(format!(
                                    "Total messages: {}",
                                    reaction_messages.len()
                                )))
                                .description(description),
                        )
                        .ephemeral(true),
                )
                .await?;
            }
        }
    }

    return Ok(());
}
