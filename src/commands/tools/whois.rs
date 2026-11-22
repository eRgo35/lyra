use serde_json::Value;

use super::lookup::{get_json, reply};
use crate::{Context, Error};

/// Shows registration data for a domain
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn whois(
    ctx: Context<'_>,
    #[description = "Domain name, e.g. example.com"] domain: String,
) -> Result<(), Error> {
    let result = fetch_domain(&ctx.data().http_client, RDAP_API, &domain).await;
    reply(ctx, "Whois", result).await
}

/// rdap.org redirects each query to the registry responsible for it.
pub const RDAP_API: &str = "https://rdap.org";

pub(super) const SERVICE: &str = "the RDAP service";

/// Lower-cased domain name, or what is expected.
fn domain_name(input: &str) -> Result<String, String> {
    let domain = input.trim().trim_end_matches('.').to_ascii_lowercase();
    let valid = domain.contains('.')
        && !domain.starts_with('.')
        && !domain.contains("..")
        && domain
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');
    if valid {
        Ok(domain)
    } else {
        Err(format!(
            "\"{}\" is not a domain name (e.g. example.com)!",
            input.trim()
        ))
    }
}

/// RDAP registration data for `domain` as an embed title and description.
pub async fn fetch_domain(
    client: &reqwest::Client,
    base_url: &str,
    domain: &str,
) -> Result<(String, String), String> {
    let domain = domain_name(domain)?;
    let data: Value = get_json(client, &format!("{base_url}/domain/{domain}"), &[], SERVICE)
        .await?
        .ok_or_else(|| format!("No registration data found for {domain}!"))?;

    let mut lines = Vec::new();
    if let Some(registrar) = find_entity(&data, "registrar").and_then(|e| vcard(e, "fn")) {
        lines.push(format!("**Registrar:** {registrar}"));
    }
    for (action, label) in [
        ("registration", "Registered"),
        ("expiration", "Expires"),
        ("last changed", "Last changed"),
    ] {
        if let Some(date) = event(&data, action) {
            lines.push(format!("**{label}:** {date}"));
        }
    }
    let status = strings(&data["status"]);
    if !status.is_empty() {
        lines.push(format!("**Status:** {}", status.join(", ")));
    }
    let nameservers: Vec<String> = data["nameservers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|ns| ns["ldhName"].as_str())
        .map(str::to_ascii_lowercase)
        .collect();
    if !nameservers.is_empty() {
        lines.push(format!("**Name servers:** {}", nameservers.join(", ")));
    }
    if lines.is_empty() {
        lines.push("The registry returned no details.".to_string());
    }

    Ok((domain, lines.join("\n")))
}

/// The date (YYYY-MM-DD) of the first RDAP event with `action`.
fn event(data: &Value, action: &str) -> Option<String> {
    data["events"]
        .as_array()?
        .iter()
        .find(|e| e["eventAction"] == action)?["eventDate"]
        .as_str()
        .map(|date| date.chars().take(10).collect())
}

fn strings(value: &Value) -> Vec<&str> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

/// The first entity (searching nested entities too) that has `role`.
pub(super) fn find_entity<'a>(data: &'a Value, role: &str) -> Option<&'a Value> {
    let entities = data["entities"].as_array()?;
    entities
        .iter()
        .find(|e| strings(&e["roles"]).contains(&role))
        .or_else(|| entities.iter().find_map(|e| find_entity(e, role)))
}

/// A text property (`fn`, `email`, ...) from an entity's jCard.
pub(super) fn vcard<'a>(entity: &'a Value, property: &str) -> Option<&'a str> {
    entity["vcardArray"][1]
        .as_array()?
        .iter()
        .find(|p| p[0] == property)?[3]
        .as_str()
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_names_are_normalised_and_validated() {
        assert_eq!(domain_name(" Example.COM. "), Ok("example.com".to_string()));
        assert!(domain_name("localhost").is_err());
        assert!(domain_name("../ip/1.1.1.1").is_err());
        assert!(domain_name("a..b").is_err());
    }

    #[test]
    fn entities_are_found_by_role_at_any_depth() {
        let data = serde_json::json!({"entities": [
            {"roles": ["registrant"], "vcardArray": ["vcard", [["fn", {}, "text", "Org"]]],
             "entities": [{"roles": ["abuse"],
                           "vcardArray": ["vcard", [["email", {}, "text", "abuse@org"]]]}]}
        ]});
        assert_eq!(
            vcard(find_entity(&data, "registrant").unwrap(), "fn"),
            Some("Org")
        );
        assert_eq!(
            vcard(find_entity(&data, "abuse").unwrap(), "email"),
            Some("abuse@org")
        );
        assert!(find_entity(&data, "registrar").is_none());
    }
}
