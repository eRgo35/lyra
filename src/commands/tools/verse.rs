use crate::{
    commands::embeds::{embed, error_embed},
    Context, Error,
};
use poise::CreateReply;
use serde::{Deserialize, Serialize};
use tracing::warn;
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
    let response = ctx
        .data()
        .http_client
        .get(format!(
            "https://bible-api.com/{}?translation={}",
            data, translation
        ))
        .send()
        .await;

    let error = match response {
        Err(e) => {
            warn!("Bible API request failed: {e}");
            "Could not reach the Bible API!".to_string()
        }
        Ok(response) => match response.status() {
            reqwest::StatusCode::OK => match response.json::<APIResponse>().await {
                Ok(parsed) if parsed.text.len() > 4000 => "Quoted text is too long!".to_string(),
                Ok(parsed) => {
                    ctx.send(
                        CreateReply::default().embed(
                            embed(
                                ctx,
                                &parsed.translation_name,
                                &parsed.text,
                                &parsed.reference,
                            )
                            .await?,
                        ),
                    )
                    .await?;
                    return Ok(());
                }
                Err(e) => {
                    warn!("Could not parse Bible API response: {e}");
                    "Could not read the Bible API response!".to_string()
                }
            },
            reqwest::StatusCode::NOT_FOUND => format!("Verse \"{}\" not found!", verse),
            status => {
                warn!("Bible API returned {status}");
                format!("The Bible API returned an error ({status})!")
            }
        },
    };

    ctx.send(CreateReply::default().embed(error_embed(ctx, &error).await?))
        .await?;

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
struct APIResponse {
    reference: String,
    text: String,
    translation_name: String,
    translation_note: String,
}
