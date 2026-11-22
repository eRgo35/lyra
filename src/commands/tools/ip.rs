use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use serde_json::Value;

use super::lookup::{get_json, reply};
use super::whois::{find_entity, vcard, RDAP_API, SERVICE};
use crate::{Context, Error};

/// Shows who owns an IP address or a hostname's address
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn ip(
    ctx: Context<'_>,
    #[description = "IP address or hostname"] target: String,
) -> Result<(), Error> {
    let result = match resolve(&target).await {
        Ok(address) => fetch_ip(&ctx.data().http_client, RDAP_API, address)
            .await
            .map(|(title, description)| {
                let target = target.trim();
                if target == address.to_string() {
                    (title, description)
                } else {
                    (format!("{target} → {title}"), description)
                }
            }),
        Err(message) => Err(message),
    };
    reply(ctx, "IP Info", result).await
}

/// Parses `target` as an address or resolves it as a hostname. Only public addresses
/// are returned: looking up private ones would reveal the bot host's local network.
pub async fn resolve(target: &str) -> Result<IpAddr, String> {
    let target = target.trim();
    let address = match target.parse::<IpAddr>() {
        Ok(address) => address,
        Err(_) => {
            let hostname = !target.is_empty()
                && target
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');
            if !hostname {
                return Err(format!("\"{target}\" is not an IP address or hostname!"));
            }
            tokio::net::lookup_host((target, 0))
                .await
                .ok()
                .and_then(|mut addresses| addresses.next())
                .map(|socket| socket.ip())
                .ok_or_else(|| format!("Could not resolve {target}!"))?
        }
    };

    if is_public(address) {
        Ok(address)
    } else {
        Err(format!("{address} is a private or reserved address!"))
    }
}

fn is_public(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => is_public_v4(v4),
            None => is_public_v6(v6),
        },
    }
}

fn is_public_v4(a: Ipv4Addr) -> bool {
    let [first, second, ..] = a.octets();
    let shared = first == 100 && (64..128).contains(&second); // 100.64.0.0/10 (CGNAT)
    !(a.is_private()
        || a.is_loopback()
        || a.is_link_local()
        || a.is_unspecified()
        || a.is_broadcast()
        || a.is_documentation()
        || a.is_multicast()
        || shared
        || first == 0
        || first >= 240)
}

fn is_public_v6(a: Ipv6Addr) -> bool {
    let first = a.segments()[0];
    let unique_local = first & 0xfe00 == 0xfc00; // fc00::/7
    let link_local = first & 0xffc0 == 0xfe80; // fe80::/10
    let documentation = first == 0x2001 && a.segments()[1] == 0x0db8;
    !(a.is_loopback()
        || a.is_unspecified()
        || a.is_multicast()
        || unique_local
        || link_local
        || documentation)
}

/// RDAP network data for `address` as an embed title and description.
pub async fn fetch_ip(
    client: &reqwest::Client,
    base_url: &str,
    address: IpAddr,
) -> Result<(String, String), String> {
    let data: Value = get_json(client, &format!("{base_url}/ip/{address}"), &[], SERVICE)
        .await?
        .ok_or_else(|| format!("No registration data found for {address}!"))?;

    let mut lines = Vec::new();
    if let Some(name) = data["name"].as_str() {
        lines.push(format!("**Network:** {name}"));
    }
    if let (Some(start), Some(end)) = (data["startAddress"].as_str(), data["endAddress"].as_str()) {
        lines.push(format!("**Range:** {start} – {end}"));
    }
    if let Some(country) = data["country"].as_str() {
        lines.push(format!("**Country:** {country}"));
    }
    if let Some(owner) = find_entity(&data, "registrant").and_then(|e| vcard(e, "fn")) {
        lines.push(format!("**Owner:** {owner}"));
    }
    if let Some(abuse) = find_entity(&data, "abuse").and_then(|e| vcard(e, "email")) {
        lines.push(format!("**Abuse contact:** {abuse}"));
    }
    if lines.is_empty() {
        lines.push("The registry returned no details.".to_string());
    }

    Ok((address.to_string(), lines.join("\n")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn private_and_reserved_addresses_are_refused() {
        for target in [
            "127.0.0.1",
            "10.1.2.3",
            "192.168.0.10",
            "100.64.0.1",
            "169.254.169.254",
            "::1",
            "fd00::1",
            "::ffff:10.0.0.1",
            "localhost",
        ] {
            assert!(resolve(target).await.is_err(), "{target} was allowed");
        }
    }

    #[tokio::test]
    async fn public_addresses_pass_through() {
        assert_eq!(resolve(" 1.1.1.1 ").await, Ok("1.1.1.1".parse().unwrap()));
        assert_eq!(
            resolve("2606:4700:4700::1111").await,
            Ok("2606:4700:4700::1111".parse().unwrap())
        );
    }

    #[tokio::test]
    async fn junk_is_not_resolved() {
        assert!(resolve("not a host!").await.is_err());
        assert!(resolve("").await.is_err());
    }
}
