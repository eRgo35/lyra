use serde::Deserialize;

use super::lookup::{get_json, reply, Card};
use crate::{Context, Error};

/// Shows whether a Minecraft server is online, its MOTD and player count
#[poise::command(prefix_command, slash_command, category = "Minecraft")]
pub async fn mcstatus(
    ctx: Context<'_>,
    #[description = "Server address, e.g. play.example.net or example.net:25566"] server: String,
) -> Result<(), Error> {
    let result = fetch_status(&ctx.data().http_client, MCSRVSTAT_API, &server).await;
    reply(ctx, "Minecraft server", result).await
}

/// mcsrvstat.us (Java edition), no key needed; it wants a User-Agent.
pub const MCSRVSTAT_API: &str = "https://api.mcsrvstat.us";

/// Lower-cased `host` or `host:port`, or what is expected.
fn address(input: &str) -> Result<String, String> {
    let address = input.trim().to_ascii_lowercase();
    let (host, port) = match address.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (address.as_str(), None),
    };
    let host_ok = !host.is_empty()
        && host.len() <= 253
        && !host.starts_with('.')
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');
    let port_ok = port.is_none_or(|p| p.parse::<u16>().is_ok_and(|p| p > 0));
    if host_ok && port_ok {
        Ok(address)
    } else {
        Err(format!(
            "\"{}\" is not a server address (e.g. play.example.net or example.net:25566)!",
            input.trim()
        ))
    }
}

#[derive(Deserialize)]
struct Status {
    online: bool,
    hostname: Option<String>,
    version: Option<String>,
    software: Option<String>,
    motd: Option<Lines>,
    players: Option<Players>,
}

#[derive(Deserialize)]
struct Lines {
    #[serde(default)]
    clean: Vec<String>,
}

#[derive(Deserialize)]
struct Players {
    online: u64,
    max: u64,
    #[serde(default)]
    list: Vec<Player>,
}

#[derive(Deserialize)]
struct Player {
    name: String,
}

const LISTED_PLAYERS: usize = 10;

/// Status of the Java server at `server` as an embed card.
pub async fn fetch_status(
    client: &reqwest::Client,
    base_url: &str,
    server: &str,
) -> Result<Card, String> {
    let address = address(server)?;
    let status: Status = get_json(
        client,
        &format!("{base_url}/3/{address}"),
        &[],
        "mcsrvstat.us",
    )
    .await?
    .ok_or_else(|| format!("No status for {address}!"))?;

    if !status.online {
        return Ok(Card {
            title: address,
            description: "🔴 Offline, or not a Minecraft server.".to_string(),
            ..Card::default()
        });
    }

    let mut lines = vec!["🟢 Online".to_string()];
    if let Some(motd) = status.motd.filter(|m| !m.clean.is_empty()) {
        let motd: Vec<_> = motd
            .clean
            .iter()
            .map(|l| l.trim().replace('`', "'"))
            .collect();
        lines.push(format!("```\n{}\n```", motd.join("\n")));
    }
    if let Some(players) = &status.players {
        lines.push(format!("**Players:** {} / {}", players.online, players.max));
        if !players.list.is_empty() {
            let mut names: Vec<_> = players
                .list
                .iter()
                .take(LISTED_PLAYERS)
                .map(|p| p.name.as_str())
                .collect();
            if players.list.len() > LISTED_PLAYERS {
                names.push("…");
            }
            lines.push(format!("**Online now:** {}", names.join(", ")));
        }
    }
    let version = [status.software.as_deref(), status.version.as_deref()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    if !version.is_empty() {
        lines.push(format!("**Version:** {version}"));
    }

    Ok(Card {
        title: status.hostname.unwrap_or_else(|| address.clone()),
        description: lines.join("\n"),
        thumbnail: Some(format!("{base_url}/icon/{address}")),
        image: None,
    })
}

#[cfg(test)]
mod tests {
    use super::address;

    #[test]
    fn addresses_are_normalised_and_validated() {
        assert_eq!(
            address(" Play.Example.NET "),
            Ok("play.example.net".to_string())
        );
        assert_eq!(
            address("example.net:25566"),
            Ok("example.net:25566".to_string())
        );
        assert_eq!(address("1.2.3.4"), Ok("1.2.3.4".to_string()));
        for bad in [
            "",
            "example.net:0",
            "example.net:99999",
            "a b",
            "../icon",
            "x:y:z",
        ] {
            assert!(address(bad).is_err(), "{bad:?}");
        }
    }
}
