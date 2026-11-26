//! `/soundboard`: a server's custom sounds as buttons that play on top of the queue.

use std::sync::Arc;
use std::time::Duration;

use poise::serenity_prelude::{
    ButtonStyle, ComponentInteraction, ComponentInteractionCollector, CreateActionRow,
    CreateButton, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
    EditInteractionResponse, GuildId, UserId,
};
use poise::CreateReply;
use songbird::input::{AuxMetadata, Compose};
use songbird::tracks::Track;

use super::store::{sound_name, Sound};
use crate::commands::embeds::{embed, error_embed};
use crate::commands::music::notifier::TrackErrorNotifier;
use crate::{Context, Error};

/// Buttons stay clickable this long after `soundboard show`.
const BUTTONS_TIMEOUT: Duration = Duration::from_secs(10 * 60);
/// Four rows of five; the fifth row holds the page buttons.
const PER_PAGE: usize = 20;

/// Your server's soundboard: show, add, remove or play sounds
#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    category = "Music",
    aliases("sb"),
    subcommands("show", "add", "remove", "play_sound")
)]
pub async fn soundboard(ctx: Context<'_>) -> Result<(), Error> {
    // Only reachable as a prefix command (`!soundboard`); slash needs a subcommand.
    show_board(ctx).await
}

/// Shows the soundboard as buttons
#[poise::command(prefix_command, slash_command, guild_only, category = "Music")]
pub async fn show(ctx: Context<'_>) -> Result<(), Error> {
    show_board(ctx).await
}

/// Adds a sound from a URL or a search
#[poise::command(prefix_command, slash_command, guild_only, category = "Music")]
pub async fn add(
    ctx: Context<'_>,
    #[description = "Name for the button (letters, digits, - and _)"] name: String,
    #[description = "URL or search terms"]
    #[rest]
    source: String,
) -> Result<(), Error> {
    let name = match sound_name(&name) {
        Ok(name) => name,
        Err(msg) => return reply_error(ctx, &msg).await,
    };
    let source = source.trim();
    if source.is_empty() {
        return reply_error(ctx, "Give a URL or search terms for the sound!").await;
    }
    ctx.defer().await?;

    // Resolve once now, so a bad source fails here rather than on every click, and the
    // button plays the same thing each time.
    let query = if source.starts_with("http://") || source.starts_with("https://") {
        source.to_string()
    } else {
        format!("ytsearch:{source}")
    };
    let metadata = crate::http::ytdl(ctx.data().media_client.clone(), query)
        .aux_metadata()
        .await;
    let Some((url, title)) = metadata.ok().and_then(|m| {
        let url = m.source_url?;
        Some((url, m.title.unwrap_or_else(|| "Unknown Title".to_string())))
    }) else {
        return reply_error(
            ctx,
            &format!("Could not find anything playable for \"{source}\"!"),
        )
        .await;
    };

    let sound = Sound {
        name: name.clone(),
        title: title.clone(),
        source: url.clone(),
        added_by: ctx.author().id.get(),
    };
    if let Err(msg) = ctx.data().sounds.add(guild(ctx), sound).await {
        return reply_error(ctx, &msg).await;
    }
    let description = format!("[{title}]({url})");
    ctx.send(
        CreateReply::default()
            .embed(embed(ctx, "Soundboard", &description, &format!("Added `{name}`")).await?),
    )
    .await?;
    Ok(())
}

/// Removes a sound (yours, or any with Manage Server)
#[poise::command(prefix_command, slash_command, guild_only, category = "Music")]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "Name of the sound"] name: String,
) -> Result<(), Error> {
    let name = name.trim().to_lowercase();
    let author = ctx.author().id.get();
    let manager = can_manage_guild(ctx).await;
    match ctx
        .data()
        .sounds
        .remove(guild(ctx), &name, |s| manager || s.added_by == author)
        .await
    {
        Ok(sound) => {
            let title = format!("Removed `{}`", sound.name);
            ctx.send(
                CreateReply::default().embed(embed(ctx, "Soundboard", &sound.title, &title).await?),
            )
            .await?;
            Ok(())
        }
        Err(msg) => reply_error(ctx, &msg).await,
    }
}

/// Plays a sound by name
#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    category = "Music",
    rename = "play"
)]
pub async fn play_sound(
    ctx: Context<'_>,
    #[description = "Name of the sound"] name: String,
) -> Result<(), Error> {
    let name = name.trim().to_lowercase();
    let result = match ctx.data().sounds.get(guild(ctx), &name).await {
        Ok(sound) => play(ctx, &sound, ctx.author().id).await.map(|()| sound),
        Err(msg) => Err(msg),
    };
    match result {
        Ok(sound) => {
            let title = format!("▶ {}", sound.name);
            ctx.send(
                CreateReply::default().embed(embed(ctx, "Soundboard", &sound.title, &title).await?),
            )
            .await?;
            Ok(())
        }
        Err(msg) => reply_error(ctx, &msg).await,
    }
}

fn guild(ctx: Context<'_>) -> u64 {
    ctx.guild_id().expect("guild_only").get()
}

async fn reply_error(ctx: Context<'_>, msg: &str) -> Result<(), Error> {
    ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await?))
        .await?;
    Ok(())
}

async fn can_manage_guild(ctx: Context<'_>) -> bool {
    let Some(member) = ctx.author_member().await else {
        return false;
    };
    // Slash interactions carry the resolved permissions; prefix commands use the cache.
    if let Some(permissions) = member.permissions {
        return permissions.manage_guild();
    }
    #[allow(deprecated)]
    ctx.guild()
        .map(|guild| guild.member_permissions(&member).manage_guild())
        .unwrap_or(false)
}

async fn show_board(ctx: Context<'_>) -> Result<(), Error> {
    let sounds = match ctx.data().sounds.list(guild(ctx)).await {
        Ok(sounds) => sounds,
        Err(msg) => return reply_error(ctx, &msg).await,
    };
    if sounds.is_empty() {
        return reply_error(
            ctx,
            "The soundboard is empty. Add a sound with `/soundboard add <name> <url or search>`.",
        )
        .await;
    }

    let base = embed(ctx, "Soundboard", "", "").await?;
    let mut page = 0;
    let handle = ctx
        .send(
            CreateReply::default()
                .embed(page_embed(base.clone(), &sounds, page))
                .components(page_buttons(&sounds, page)),
        )
        .await?;
    let message = handle.message().await?;

    while let Some(click) = ComponentInteractionCollector::new(ctx.serenity_context())
        .message_id(message.id)
        .timeout(BUTTONS_TIMEOUT)
        .await
    {
        let id = click.data.custom_id.as_str();
        if let Some(target) = id.strip_prefix("page:").and_then(|p| p.parse().ok()) {
            page = target;
            let update = CreateInteractionResponseMessage::new()
                .embed(page_embed(base.clone(), &sounds, page))
                .components(page_buttons(&sounds, page));
            click
                .create_response(ctx, CreateInteractionResponse::UpdateMessage(update))
                .await?;
        } else if let Some(name) = id.strip_prefix("play:") {
            play_clicked(ctx, &click, name).await?;
        }
    }

    // Timed out: drop the buttons so nobody clicks dead ones.
    let expired = format!(
        "{}\n\nButtons expired; run `/soundboard show` again.",
        page_text(&sounds, page)
    );
    handle
        .edit(
            ctx,
            CreateReply::default()
                .embed(base.description(expired))
                .components(vec![]),
        )
        .await?;
    Ok(())
}

/// Plays the clicked sound and tells only the clicker how it went.
async fn play_clicked(
    ctx: Context<'_>,
    click: &ComponentInteraction,
    name: &str,
) -> Result<(), Error> {
    let defer =
        CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new().ephemeral(true));
    click.create_response(ctx, defer).await?;

    let result = match ctx.data().sounds.get(guild(ctx), name).await {
        Ok(sound) => play(ctx, &sound, click.user.id)
            .await
            .map(|()| format!("▶ **{}**: {}", sound.name, sound.title)),
        Err(msg) => Err(msg),
    };
    let text = result.unwrap_or_else(|msg| format!("⚠️ {msg}"));
    click
        .edit_response(ctx, EditInteractionResponse::new().content(text))
        .await?;
    Ok(())
}

/// Plays `sound` on top of the queue, joining `user`'s voice channel first if the bot
/// is not in one yet.
async fn play(ctx: Context<'_>, sound: &Sound, user: UserId) -> Result<(), String> {
    let serenity_ctx = ctx.serenity_context();
    let guild_id: GuildId = ctx.guild_id().expect("guild_only");
    let manager = songbird::get(serenity_ctx)
        .await
        .expect("Songbird client placed at init")
        .clone();

    let call = match manager.get(guild_id) {
        Some(call) => call,
        None => {
            let channel = serenity_ctx
                .cache
                .guild(guild_id)
                .and_then(|g| g.voice_states.get(&user).and_then(|v| v.channel_id))
                .ok_or("Join a voice channel first!")?;
            let call = manager
                .join(guild_id, channel)
                .await
                .map_err(|_| "I could not join your voice channel!".to_string())?;
            TrackErrorNotifier::attach(
                &mut *call.lock().await,
                serenity_ctx.http.clone(),
                ctx.channel_id(),
            );
            call
        }
    };

    let metadata = AuxMetadata {
        title: Some(sound.title.clone()),
        ..AuxMetadata::default()
    };
    let src = crate::http::ytdl(ctx.data().media_client.clone(), sound.source.clone())
        .with_metadata(metadata.clone());
    let _ = call
        .lock()
        .await
        .play(Track::new_with_data(src.into(), Arc::new(metadata)));
    Ok(())
}

fn pages(sounds: &[Sound]) -> usize {
    sounds.len().div_ceil(PER_PAGE).max(1)
}

fn page_text(sounds: &[Sound], page: usize) -> String {
    sounds
        .iter()
        .skip(page * PER_PAGE)
        .take(PER_PAGE)
        .map(|s| format!("`{}` {}", s.name, s.title))
        .collect::<Vec<_>>()
        .join("\n")
}

fn page_embed(base: CreateEmbed, sounds: &[Sound], page: usize) -> CreateEmbed {
    let mut embed = base.description(page_text(sounds, page));
    if pages(sounds) > 1 {
        embed = embed.title(format!("Page {} of {}", page + 1, pages(sounds)));
    }
    embed
}

/// Up to four rows of five sound buttons, plus ◀ ▶ when there is more than one page.
fn page_buttons(sounds: &[Sound], page: usize) -> Vec<CreateActionRow> {
    let buttons: Vec<CreateButton> = sounds
        .iter()
        .skip(page * PER_PAGE)
        .take(PER_PAGE)
        .map(|s| {
            CreateButton::new(format!("play:{}", s.name))
                .label(&s.name)
                .style(ButtonStyle::Secondary)
        })
        .collect();
    let mut rows: Vec<CreateActionRow> = buttons
        .chunks(5)
        .map(|row| CreateActionRow::Buttons(row.to_vec()))
        .collect();

    let total = pages(sounds);
    if total > 1 {
        rows.push(CreateActionRow::Buttons(vec![
            CreateButton::new(format!("page:{}", page.saturating_sub(1)))
                .label("◀")
                .style(ButtonStyle::Primary)
                .disabled(page == 0),
            CreateButton::new(format!("page:{}", (page + 1).min(total - 1)))
                .label("▶")
                .style(ButtonStyle::Primary)
                .disabled(page + 1 >= total),
        ]));
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sounds(n: usize) -> Vec<Sound> {
        (0..n)
            .map(|i| Sound {
                name: format!("s{i:03}"),
                title: format!("Sound {i}"),
                source: String::new(),
                added_by: 0,
            })
            .collect()
    }

    fn rows_json(rows: Vec<CreateActionRow>) -> Vec<serde_json::Value> {
        rows.into_iter()
            .map(|r| serde_json::to_value(r).unwrap())
            .collect()
    }

    #[test]
    fn a_single_page_has_no_page_buttons() {
        let rows = rows_json(page_buttons(&sounds(7), 0));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["components"].as_array().unwrap().len(), 5);
        assert_eq!(rows[1]["components"][1]["custom_id"], "play:s006");
    }

    #[test]
    fn pages_fit_discords_five_rows_of_five() {
        let all = sounds(100);
        assert_eq!(pages(&all), 5);
        for page in 0..5 {
            let rows = rows_json(page_buttons(&all, page));
            assert_eq!(rows.len(), 5, "page {page}");
            assert!(rows
                .iter()
                .all(|r| r["components"].as_array().unwrap().len() <= 5));
        }
        let first = rows_json(page_buttons(&all, 0));
        assert_eq!(first[4]["components"][0]["disabled"], true);
        assert_eq!(first[4]["components"][1]["custom_id"], "page:1");
        let last = rows_json(page_buttons(&all, 4));
        assert_eq!(last[0]["components"][0]["custom_id"], "play:s080");
        assert_eq!(last[4]["components"][1]["disabled"], true);
    }

    #[test]
    fn page_text_lists_that_page_only() {
        let all = sounds(25);
        assert_eq!(
            page_text(&all, 1),
            "`s020` Sound 20\n`s021` Sound 21\n`s022` Sound 22\n`s023` Sound 23\n`s024` Sound 24"
        );
    }
}
