use crate::{
    commands::embeds::{embed, error_embed},
    Context, Error,
};
use poise::CreateReply;
use serde::{Deserialize, Serialize};
use url::form_urlencoded;

/// Reference Bible by verse
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn verse(
    ctx: Context<'_>,
    #[description = "Latin?"]
    #[flag]
    latin: bool,
    #[description = "BOOK+CHAPTER:VERSE"]
    #[rest]
    verse: String,
) -> Result<(), Error> {
    let data: String = form_urlencoded::byte_serialize(verse.as_bytes()).collect();
    let translation = if latin { "clementine" } else { "web" };
    let client = reqwest::Client::new();
    let response = client
        .get(format!(
            "https://bible-api.com/{}?translation={}",
            data, translation
        ))
        .send()
        .await
        .unwrap();

    match response.status() {
        reqwest::StatusCode::OK => {
            match response.json::<APIResponse>().await {
                Ok(parsed) => {
                    if parsed.text.len() > 4000 {
                        ctx.send(
                            CreateReply::default()
                                .embed(error_embed(ctx, "Quoted text is too long!").await.unwrap()),
                        )
                        .await?;
                        return Ok(());
                    }
                    ctx.send(
                        CreateReply::default().embed(
                            embed(
                                ctx,
                                &parsed.translation_name,
                                &parsed.text,
                                &parsed.reference,
                            )
                            .await
                            .unwrap(),
                        ),
                    )
                    .await?;
                }
                Err(err) => println!("Something is messed up! {:?}", err),
            };
        }
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
struct APIResponse {
    reference: String,
    text: String,
    translation_name: String,
    translation_note: String,
}
