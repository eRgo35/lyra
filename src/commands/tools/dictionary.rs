use poise::CreateReply;
use serde::{Deserialize, Serialize};
use tracing::warn;
use url::form_urlencoded;

use crate::{
    commands::embeds::{embed, error_embed},
    Context, Error,
};

/// Explains provided query
#[poise::command(prefix_command, slash_command, aliases("dict"), category = "Tools")]
pub async fn dictionary(
    ctx: Context<'_>,
    #[description = "Word you're looking for"]
    #[rest]
    word: String,
) -> Result<(), Error> {
    match define(&ctx.data().http_client, DICTIONARY_API, &word).await {
        Ok((title, description)) => {
            ctx.send(
                CreateReply::default().embed(embed(ctx, "Dictionary", &description, &title).await?),
            )
            .await?;
        }
        Err(msg) => {
            ctx.send(CreateReply::default().embed(error_embed(ctx, &msg).await?))
                .await?;
        }
    }

    Ok(())
}

pub const DICTIONARY_API: &str = "https://api.dictionaryapi.dev/api/v2/entries/en";

/// Looks `word` up at `base_url` (the dictionaryapi.dev entries endpoint).
///
/// Returns the embed title and description, or the message to show the user.
pub async fn define(
    client: &reqwest::Client,
    base_url: &str,
    word: &str,
) -> Result<(String, String), String> {
    let data: String = form_urlencoded::byte_serialize(word.as_bytes()).collect();
    let not_found = || format!("No definitions found for \"{}\"!", word);

    let response = client
        .get(format!("{}/{}", base_url, data))
        .send()
        .await
        .map_err(|e| {
            warn!("Dictionary request failed: {e}");
            "Could not reach the dictionary!".to_string()
        })?;

    match response.status() {
        reqwest::StatusCode::OK => {
            let parsed = response.json::<Vec<Word>>().await.map_err(|e| {
                warn!("Could not parse dictionary response: {e}");
                "Could not read the dictionary response!".to_string()
            })?;
            parsed.first().map(format_entry).ok_or_else(not_found)
        }
        reqwest::StatusCode::NOT_FOUND => Err(not_found()),
        status => {
            warn!("Dictionary API returned {status}");
            Err(format!("The dictionary returned an error ({status})!"))
        }
    }
}

/// Embed title (word and phonetic) and description (first definition per part of speech).
fn format_entry(entry: &Word) -> (String, String) {
    let phonetic = entry
        .phonetic
        .as_deref()
        .or_else(|| entry.phonetics.iter().find_map(|p| p.text.as_deref()));

    let title = match phonetic {
        Some(phonetic) => format!("{} {}", entry.word, phonetic),
        None => entry.word.clone(),
    };

    let mut description = entry
        .meanings
        .iter()
        .filter_map(|meaning| {
            let definition = meaning.definitions.first()?;
            Some(format!(
                "**{}**: {}",
                meaning.partOfSpeech, definition.definition
            ))
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Discord limits embed descriptions to 4096 characters.
    if description.chars().count() > 4000 {
        description = description.chars().take(4000).collect::<String>() + "…";
    }

    (title, description)
}

#[derive(Serialize, Deserialize, Debug)]
struct Definition {
    definition: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct Meaning {
    partOfSpeech: String,
    definitions: Vec<Definition>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct Phonetic {
    text: Option<String>,
    audio: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Word {
    word: String,
    phonetic: Option<String>,
    phonetics: Vec<Phonetic>,
    meanings: Vec<Meaning>,
}

#[cfg(test)]
mod tests {
    use super::{format_entry, Word};

    #[test]
    fn format_entry_uses_first_definition_per_part_of_speech_and_phonetics_fallback() {
        let json = r#"{"word": "hello", "phonetics": [{"audio": ""}, {"text": "/həˈləʊ/"}],
            "meanings": [
                {"partOfSpeech": "noun", "definitions": [{"definition": "A greeting."}, {"definition": "Other."}]},
                {"partOfSpeech": "verb", "definitions": [{"definition": "To greet."}]},
                {"partOfSpeech": "empty", "definitions": []}
            ]}"#;
        let word: Word = serde_json::from_str(json).unwrap();

        let (title, description) = format_entry(&word);
        assert_eq!(title, "hello /həˈləʊ/");
        assert_eq!(description, "**noun**: A greeting.\n**verb**: To greet.");
    }
}
