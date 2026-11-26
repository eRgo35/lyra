use crate::{Context, Error};

use poise::serenity_prelude::{
    Color, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, Timestamp,
};
use poise::CreateReply;
use std::fmt::Display;
use std::sync::LazyLock;

/// `DETAILED_ERRORS=true` (or `1`): error messages also show the underlying error,
/// for diagnosing a deployment. Off by default, since it can expose internals.
static DETAILED_ERRORS: LazyLock<bool> = LazyLock::new(|| {
    matches!(
        std::env::var("DETAILED_ERRORS").as_deref(),
        Ok("1" | "true")
    )
});

/// The longest detail shown; the end is kept, since that is where yt-dlp puts its error.
const MAX_DETAIL: usize = 1000;

/// `msg`, followed by `detail` in a code block when `DETAILED_ERRORS` is on.
pub fn with_detail(msg: &str, detail: impl Display) -> String {
    append_detail(msg, &detail.to_string(), *DETAILED_ERRORS)
}

fn append_detail(msg: &str, detail: &str, enabled: bool) -> String {
    let detail = detail.trim().replace("```", "'''");
    if !enabled || detail.is_empty() {
        return msg.to_string();
    }
    let start = (detail.len().saturating_sub(MAX_DETAIL)..=detail.len())
        .find(|&i| detail.is_char_boundary(i))
        .unwrap_or(0);
    let ellipsis = if start > 0 { "…" } else { "" };
    format!("{msg}\n```\n{ellipsis}{}\n```", &detail[start..])
}

pub async fn fail(ctx: Context<'_>, err: String) -> Result<(), Error> {
    ctx.send(CreateReply::default().embed(error_embed(ctx, &format!("Failed: {:?}", err)).await?))
        .await?;

    Ok(())
}

pub async fn error_embed(ctx: Context<'_>, msg: &str) -> Result<CreateEmbed, Error> {
    let embed = CreateEmbed::default()
        .author(
            CreateEmbedAuthor::new("Something went wrong!").icon_url(ctx.author().clone().face()),
        )
        .colour(Color::from_rgb(255, 58, 97))
        .title("Oopsie, Doopsie!")
        .description(msg)
        .timestamp(Timestamp::now())
        .footer(
            CreateEmbedFooter::new(ctx.cache().current_user().name.to_string())
                .icon_url(ctx.cache().current_user().face()),
        );

    Ok(embed)
}

pub async fn embed(
    ctx: Context<'_>,
    author: &str,
    description: &str,
    title: &str,
) -> Result<CreateEmbed, Error> {
    let embed = CreateEmbed::default()
        .author(CreateEmbedAuthor::new(author).icon_url(ctx.author().clone().face()))
        .colour(Color::from_rgb(255, 58, 97))
        .title(title)
        .description(description)
        .timestamp(Timestamp::now())
        .footer(
            CreateEmbedFooter::new(ctx.cache().current_user().name.to_string())
                .icon_url(ctx.cache().current_user().face()),
        );

    Ok(embed)
}

/// `03:45`, or `--:--` while the duration is not known yet.
pub fn format_duration(duration: Option<std::time::Duration>) -> String {
    match duration {
        Some(d) => format!("{:02}:{:02}", d.as_secs() / 60, d.as_secs() % 60),
        None => "--:--".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::append_detail;

    #[test]
    fn detail_only_when_enabled() {
        assert_eq!(append_detail("Oops!", "boom", false), "Oops!");
        assert_eq!(
            append_detail("Oops!", "boom", true),
            "Oops!\n```\nboom\n```"
        );
        assert_eq!(append_detail("Oops!", "  ", true), "Oops!");
    }

    #[test]
    fn long_detail_keeps_the_end() {
        let detail = format!("{}ERROR: the reason", "é".repeat(2000));
        let shown = append_detail("Oops!", &detail, true);
        assert!(shown.contains('…'));
        assert!(shown.ends_with("ERROR: the reason\n```"));
        assert!(shown.len() < 1100);
    }

    #[test]
    fn detail_cannot_close_the_code_block() {
        assert_eq!(
            append_detail("Oops!", "a```b", true),
            "Oops!\n```\na'''b\n```"
        );
    }
}
