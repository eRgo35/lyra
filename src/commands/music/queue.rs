use std::time::Duration;

use crate::{commands::embeds::error_embed, Context, Error};
use poise::serenity_prelude::{
    ButtonStyle, Color, ComponentInteractionCollector, CreateActionRow, CreateButton, CreateEmbed,
    CreateEmbedAuthor, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage, Timestamp,
};
use poise::CreateReply;
use songbird::input::AuxMetadata;
use songbird::tracks::{LoopState, TrackQueue};

const PER_PAGE: usize = 10;
/// The page buttons work this long after `queue`.
const BUTTONS_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// Shows next tracks in queue; \
/// aliases: queue, q
#[poise::command(
    prefix_command,
    slash_command,
    aliases("q"),
    guild_only,
    category = "Music"
)]
pub async fn queue(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().expect("guild_only");

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();

    let Some(handler_lock) = manager.get(guild_id) else {
        let msg = "I am not in a voice channel!";
        ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await?))
            .await?;
        return Ok(());
    };
    let queue = handler_lock.lock().await.queue().clone();

    let mut page = 0;
    let (text, total) = page_of(ctx, &queue, page).await;
    let handle = ctx
        .send(
            CreateReply::default()
                .embed(embed(ctx, text))
                .components(buttons(page, total)),
        )
        .await?;
    if total <= 1 {
        return Ok(());
    }

    let message = handle.message().await?;
    while let Some(click) = ComponentInteractionCollector::new(ctx.serenity_context())
        .message_id(message.id)
        .timeout(BUTTONS_TIMEOUT)
        .await
    {
        let Some(target) = click
            .data
            .custom_id
            .strip_prefix("queue:")
            .and_then(|p| p.parse().ok())
        else {
            continue;
        };
        // The queue moves on meanwhile, so each page is read again.
        let (text, total) = page_of(ctx, &queue, target).await;
        page = target.min(total - 1);
        let update = CreateInteractionResponseMessage::new()
            .embed(embed(ctx, text))
            .components(buttons(page, total));
        click
            .create_response(ctx, CreateInteractionResponse::UpdateMessage(update))
            .await?;
    }

    // Timed out: drop the buttons so nobody clicks dead ones.
    handle
        .edit(ctx, CreateReply::default().components(vec![]))
        .await?;
    Ok(())
}

/// Page `page` of `queue` (clamped to the last) and the number of pages.
async fn page_of(ctx: Context<'_>, queue: &TrackQueue, page: usize) -> (String, usize) {
    let tracks = queue.current_queue();
    let lines: Vec<String> = tracks
        .iter()
        .enumerate()
        .map(|(index, track)| line(index, &track.data::<AuxMetadata>()))
        .collect();

    let repeat = if ctx
        .data()
        .repeat_all
        .is_enabled(ctx.guild_id().expect("guild_only"))
    {
        Some("all".to_string())
    } else if let Some(track) = tracks.first() {
        match track.get_info().await.map(|info| info.loops) {
            Ok(LoopState::Infinite) => Some("one".to_string()),
            Ok(LoopState::Finite(n)) if n.get() > 0 => Some(format!("this track {n} more times")),
            _ => None,
        }
    } else {
        None
    };
    render(&lines, page, repeat.as_deref())
}

fn line(index: usize, metadata: &AuxMetadata) -> String {
    let secs = metadata.duration.unwrap_or_default().as_secs();
    format!(
        "{}. {} - {} [{:02}:{:02}]",
        index,
        metadata.title.as_deref().unwrap_or("Unknown Title"),
        metadata.artist.as_deref().unwrap_or("Unknown Artist"),
        secs / 60,
        secs % 60
    )
}

/// The text of page `page` (clamped to the last) of `lines`, and the number of pages.
fn render(lines: &[String], page: usize, repeat: Option<&str>) -> (String, usize) {
    let total = lines.len().div_ceil(PER_PAGE).max(1);
    let page = page.min(total - 1);
    let mut text = lines
        .iter()
        .skip(page * PER_PAGE)
        .take(PER_PAGE)
        .map(|line| format!("{line} \n"))
        .collect::<String>();
    if total > 1 {
        text.push_str(&format!(
            "\nPage {} of {} · {} tracks",
            page + 1,
            total,
            lines.len()
        ));
    }
    if let Some(repeat) = repeat {
        text.push_str(&format!("\nRepeat: {repeat}"));
    }
    (text, total)
}

/// ◀ ▶ when there is more than one page.
fn buttons(page: usize, total: usize) -> Vec<CreateActionRow> {
    if total <= 1 {
        return vec![];
    }
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("queue:{}", page.saturating_sub(1)))
            .label("◀")
            .style(ButtonStyle::Primary)
            .disabled(page == 0),
        CreateButton::new(format!("queue:{}", (page + 1).min(total - 1)))
            .label("▶")
            .style(ButtonStyle::Primary)
            .disabled(page + 1 >= total),
    ])]
}

fn embed(ctx: Context<'_>, queue: String) -> CreateEmbed {
    CreateEmbed::default()
        .author(CreateEmbedAuthor::new("Queue").icon_url(ctx.author().face()))
        .colour(Color::from_rgb(255, 58, 97))
        .title("Now playing")
        .description(queue)
        .timestamp(Timestamp::now())
        .footer(
            CreateEmbedFooter::new(ctx.cache().current_user().name.to_string())
                .icon_url(ctx.cache().current_user().face()),
        )
}

#[cfg(test)]
mod tests {
    use super::{buttons, render};

    fn lines(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("{i}. t")).collect()
    }

    #[test]
    fn one_page_has_no_page_line_or_buttons() {
        let (text, total) = render(&lines(3), 0, None);
        assert_eq!(total, 1);
        assert_eq!(text, "0. t \n1. t \n2. t \n");
        assert!(buttons(0, total).is_empty());
        assert_eq!(
            render(&[], 0, Some("all")),
            ("\nRepeat: all".to_string(), 1)
        );
    }

    #[test]
    fn pages_hold_ten_and_clamp_to_the_last() {
        let lines = lines(25);
        let (first, total) = render(&lines, 0, None);
        assert_eq!(total, 3);
        assert!(first.starts_with("0. t") && first.contains("9. t") && !first.contains("10. t"));
        assert!(first.ends_with("Page 1 of 3 · 25 tracks"));

        let (last, _) = render(&lines, 7, Some("one"));
        assert!(last.starts_with("20. t") && last.contains("24. t"));
        assert!(last.ends_with("Page 3 of 3 · 25 tracks\nRepeat: one"));
        assert_eq!(buttons(2, total).len(), 1);
    }
}
