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
    let data: String = form_urlencoded::byte_serialize(word.as_bytes()).collect();

    let response = ctx
        .data()
        .http_client
        .get(format!(
            "https://api.dictionaryapi.dev/api/v2/entries/en/{}",
            data
        ))
        .send()
        .await;

    let error = match response {
        Err(e) => {
            warn!("Dictionary request failed: {e}");
            "Could not reach the dictionary!".to_string()
        }
        Ok(response) => match response.status() {
            reqwest::StatusCode::OK => match response.json::<Vec<Word>>().await {
                Ok(parsed) => match parsed.first() {
                    Some(entry) => {
                        let (title, description) = format_entry(entry);
                        ctx.send(
                            CreateReply::default()
                                .embed(embed(ctx, "Dictionary", &description, &title).await?),
                        )
                        .await?;
                        return Ok(());
                    }
                    None => format!("No definitions found for \"{}\"!", word),
                },
                Err(e) => {
                    warn!("Could not parse dictionary response: {e}");
                    "Could not read the dictionary response!".to_string()
                }
            },
            reqwest::StatusCode::NOT_FOUND => format!("No definitions found for \"{}\"!", word),
            status => {
                warn!("Dictionary API returned {status}");
                format!("The dictionary returned an error ({status})!")
            }
        },
    };

    ctx.send(CreateReply::default().embed(error_embed(ctx, &error).await?))
        .await?;

    Ok(())
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
