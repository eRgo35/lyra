use crate::{
    commands::embeds::{embed, error_embed, with_detail},
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
    match fetch_verse(&ctx.data().http_client, BIBLE_API, &verse, latin).await {
        Ok(found) => {
            ctx.send(
                CreateReply::default().embed(
                    embed(ctx, &found.translation_name, &found.text, &found.reference).await?,
                ),
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

pub const BIBLE_API: &str = "https://bible-api.com";

/// Longest quote that still fits an embed description (Discord allows 4096 characters).
const MAX_TEXT_LEN: usize = 4000;

/// Fetches `verse` (e.g. `john 3:16`) from `base_url` (bible-api.com), in the Latin
/// Clementine Vulgate when `latin` is set, otherwise the World English Bible.
///
/// Returns the verse, or the message to show the user.
pub async fn fetch_verse(
    client: &reqwest::Client,
    base_url: &str,
    verse: &str,
    latin: bool,
) -> Result<Verse, String> {
    let data: String = form_urlencoded::byte_serialize(verse.as_bytes()).collect();
    let translation = if latin { "clementine" } else { "web" };

    let response = client
        .get(format!("{}/{}?translation={}", base_url, data, translation))
        .send()
        .await
        .map_err(|e| {
            warn!("Bible API request failed: {e}");
            with_detail("Could not reach the Bible API!", &e)
        })?;

    match response.status() {
        reqwest::StatusCode::OK => {
            let found = response.json::<Verse>().await.map_err(|e| {
                warn!("Could not parse Bible API response: {e}");
                with_detail("Could not read the Bible API response!", &e)
            })?;
            if found.text.len() > MAX_TEXT_LEN {
                return Err("Quoted text is too long!".to_string());
            }
            Ok(found)
        }
        reqwest::StatusCode::NOT_FOUND => Err(format!("Verse \"{}\" not found!", verse)),
        status => {
            warn!("Bible API returned {status}");
            Err(format!("The Bible API returned an error ({status})!"))
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Verse {
    pub reference: String,
    pub text: String,
    pub translation_name: String,
}
