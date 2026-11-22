//! Shared plumbing for the commands that look something up over HTTP
//! (`metar`, `taf`, `weather`, `whois`, `ip`).

use poise::CreateReply;
use serde::de::DeserializeOwned;
use tracing::warn;

use crate::{
    commands::embeds::{embed, error_embed},
    Context, Error,
};

const USER_AGENT: &str = concat!(
    "lyra/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/eRgo35/lyra)"
);

/// GETs `url` and parses the JSON body.
///
/// `Ok(None)` means the service had nothing for the query (404 or 204). Errors are
/// the message to show the user; `service` names the service in them ("the RDAP service").
pub async fn get_json<T: DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    query: &[(&str, &str)],
    service: &str,
) -> Result<Option<T>, String> {
    let response = client
        .get(url)
        .query(query)
        // rdap.org (and polite API use generally) wants a User-Agent. Set per request so
        // the shared client's music streaming keeps its own headers.
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await
        .map_err(|e| {
            warn!("Request to {service} failed: {e}");
            format!("Could not reach {service}!")
        })?;

    match response.status() {
        reqwest::StatusCode::NOT_FOUND | reqwest::StatusCode::NO_CONTENT => Ok(None),
        status if status.is_success() => response.json::<T>().await.map(Some).map_err(|e| {
            warn!("Could not parse the response from {service}: {e}");
            format!("Could not read the response from {service}!")
        }),
        status => {
            warn!("{service} returned {status}");
            Err(format!("Got an error from {service} ({status})!"))
        }
    }
}

/// What a lookup shows: an embed title and description, optionally with images.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Card {
    pub title: String,
    pub description: String,
    pub thumbnail: Option<String>,
    pub image: Option<String>,
}

impl From<(String, String)> for Card {
    fn from((title, description): (String, String)) -> Self {
        Card {
            title,
            description,
            ..Card::default()
        }
    }
}

/// Replies with an embed (`author`, then the card) or the error message.
pub async fn reply(
    ctx: Context<'_>,
    author: &str,
    result: Result<impl Into<Card>, String>,
) -> Result<(), Error> {
    let embed = match result.map(Into::into) {
        Ok(card) => {
            let mut embed = embed(ctx, author, &card.description, &card.title).await?;
            if let Some(thumbnail) = card.thumbnail {
                embed = embed.thumbnail(thumbnail);
            }
            if let Some(image) = card.image {
                embed = embed.image(image);
            }
            embed
        }
        Err(message) => error_embed(ctx, &message).await?,
    };
    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}
