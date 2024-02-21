use poise::CreateReply;
use serde::{Deserialize, Serialize};
use url::form_urlencoded;

use crate::{commands::embeds::embed, Context, Error};

/// Explains provided query
#[poise::command(prefix_command, slash_command, aliases("dict"), category = "Tools")]
pub async fn dictionary(
    ctx: Context<'_>,
    #[description = "Word you're looking for"]
    #[rest]
    word: String,
) -> Result<(), Error> {
    let data: String = form_urlencoded::byte_serialize(word.as_bytes()).collect();

    let client = reqwest::Client::new();
    let response = client
        .get(format!(
            "https://api.dictionaryapi.dev/api/v2/entries/en/{}",
            data
        ))
        .send()
        .await
        .unwrap();

    match response.status() {
        reqwest::StatusCode::OK => match response.json::<Vec<Word>>().await {
            Ok(parsed) => {
                println!("{:?}", parsed);

                ctx.send(CreateReply::default().embed(embed(ctx, "", "", "").await.unwrap()))
                    .await?;
            }
            Err(err) => println!("Something is messed up! {:?}", err),
        },
        reqwest::StatusCode::UNAUTHORIZED => {
            println!("Unauthorized.. Uoops!!");
        }
        error => {
            println!("Something went wrong: {:?}", error);
        }
    }

    Ok(())
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
    phonetics: Vec<Phonetic>,
    meanings: Vec<Meaning>,
}
