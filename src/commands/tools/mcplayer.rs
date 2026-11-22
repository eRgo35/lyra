use serde::Deserialize;

use super::lookup::{get_json, reply, Card};
use crate::{Context, Error};

/// Shows a Minecraft player's UUID and skin
#[poise::command(prefix_command, slash_command, category = "Minecraft")]
pub async fn mcplayer(
    ctx: Context<'_>,
    #[description = "Minecraft (Java) username"] name: String,
) -> Result<(), Error> {
    let result = fetch_player(&ctx.data().http_client, MOJANG_API, SKIN_RENDERS, &name).await;
    reply(ctx, "Minecraft player", result).await
}

pub const MOJANG_API: &str = "https://api.mojang.com";
/// mc-heads.net renders skins by UUID.
pub const SKIN_RENDERS: &str = "https://mc-heads.net";

#[derive(Deserialize)]
struct Profile {
    id: String,
    name: String,
}

/// 3-16 letters, digits or `_`, as Mojang allows.
fn username(input: &str) -> Result<&str, String> {
    let name = input.trim();
    if (3..=16).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        Ok(name)
    } else {
        Err(format!("\"{name}\" is not a Minecraft username!"))
    }
}

/// `069a79f4...` → `069a79f4-44e9-4726-a5be-fca90e38aaf5`.
fn dashed(uuid: &str) -> String {
    if uuid.len() != 32 {
        return uuid.to_string();
    }
    format!(
        "{}-{}-{}-{}-{}",
        &uuid[..8],
        &uuid[8..12],
        &uuid[12..16],
        &uuid[16..20],
        &uuid[20..]
    )
}

/// The player's profile as an embed card with a head thumbnail and body render.
pub async fn fetch_player(
    client: &reqwest::Client,
    mojang_url: &str,
    renders_url: &str,
    name: &str,
) -> Result<Card, String> {
    let name = username(name)?;
    let profile: Profile = get_json(
        client,
        &format!("{mojang_url}/users/profiles/minecraft/{name}"),
        &[],
        "Mojang",
    )
    .await?
    .ok_or_else(|| format!("There is no Minecraft player called {name}!"))?;

    let description = format!(
        "**UUID:** `{}`\n[Download skin]({renders_url}/skin/{})",
        dashed(&profile.id),
        profile.id
    );
    Ok(Card {
        title: profile.name,
        description,
        thumbnail: Some(format!("{renders_url}/avatar/{}", profile.id)),
        image: Some(format!("{renders_url}/body/{}", profile.id)),
    })
}

#[cfg(test)]
mod tests {
    use super::{dashed, username};

    #[test]
    fn usernames_are_validated() {
        assert_eq!(username(" Notch "), Ok("Notch"));
        for bad in ["ab", "seventeen_chars__", "no spaces", "a/b", "ż_name"] {
            assert!(username(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn uuids_get_dashes() {
        assert_eq!(
            dashed("069a79f444e94726a5befca90e38aaf5"),
            "069a79f4-44e9-4726-a5be-fca90e38aaf5"
        );
    }
}
