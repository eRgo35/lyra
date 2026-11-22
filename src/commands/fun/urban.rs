use serde::Deserialize;

use crate::commands::tools::lookup::{get_json, reply};
use crate::{Context, Error};

/// Looks a word up on Urban Dictionary (shown behind spoilers)
#[poise::command(prefix_command, slash_command, aliases("ud"), category = "Fun")]
pub async fn urban(
    ctx: Context<'_>,
    #[description = "Word or phrase"]
    #[rest]
    term: String,
) -> Result<(), Error> {
    let result = fetch_urban(&ctx.data().http_client, URBAN_API, &term).await;
    reply(ctx, "Urban Dictionary", result).await
}

pub const URBAN_API: &str = "https://api.urbandictionary.com";

#[derive(Deserialize)]
struct Definitions {
    #[serde(default)]
    list: Vec<Definition>,
}

#[derive(Deserialize)]
struct Definition {
    word: String,
    definition: String,
    #[serde(default)]
    example: String,
    #[serde(default)]
    thumbs_up: i64,
    #[serde(default)]
    thumbs_down: i64,
    permalink: Option<String>,
}

const DEFINITION_LIMIT: usize = 1500;
const EXAMPLE_LIMIT: usize = 800;

/// The top definition of `term` as an embed title and description. The definition
/// and example are spoilered, since Urban Dictionary is often crude.
pub async fn fetch_urban(
    client: &reqwest::Client,
    base_url: &str,
    term: &str,
) -> Result<(String, String), String> {
    let term = term.trim();
    if term.is_empty() {
        return Err("Which word? e.g. `urban yeet`".to_string());
    }
    let found: Definitions = get_json(
        client,
        &format!("{base_url}/v0/define"),
        &[("term", term)],
        "Urban Dictionary",
    )
    .await?
    .unwrap_or(Definitions { list: Vec::new() });
    let top = found
        .list
        .into_iter()
        .next()
        .ok_or_else(|| format!("Urban Dictionary has nothing for \"{term}\"!"))?;

    let mut lines = vec![spoiler(&top.definition, DEFINITION_LIMIT)];
    if !top.example.trim().is_empty() {
        lines.push(format!(
            "\n**Example**\n{}",
            spoiler(&top.example, EXAMPLE_LIMIT)
        ));
    }
    let mut footer = format!("\n👍 {} · 👎 {}", top.thumbs_up, top.thumbs_down);
    if let Some(link) = top.permalink {
        footer.push_str(&format!(" · [Open]({link})"));
    }
    lines.push(footer);
    Ok((top.word, lines.join("\n")))
}

/// Drops Urban Dictionary's `[link]` brackets, shortens, and hides the text behind a
/// spoiler. A `||` inside would end the spoiler early, so it is broken up.
fn spoiler(text: &str, limit: usize) -> String {
    let plain: String = text.chars().filter(|c| *c != '[' && *c != ']').collect();
    let plain = plain
        .trim()
        .replace("\r\n", "\n")
        .replace("||", "|\u{200b}|");
    let shortened = if plain.chars().count() > limit {
        let cut: String = plain.chars().take(limit).collect();
        format!("{}…", cut.trim_end())
    } else {
        plain
    };
    format!("||{shortened}||")
}

#[cfg(test)]
mod tests {
    use super::spoiler;

    #[test]
    fn text_is_unbracketed_spoilered_and_shortened() {
        assert_eq!(
            spoiler(" To [discard] an [item] ", 100),
            "||To discard an item||"
        );
        assert_eq!(spoiler("a || b", 100), "||a |\u{200b}| b||");
        assert_eq!(spoiler("abcdef", 3), "||abc…||");
    }
}
