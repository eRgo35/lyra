use serde::Deserialize;

use super::lookup::{get_json, reply};
use super::metar::{icao, AVIATION_WEATHER_API};
use crate::{Context, Error};

/// Shows the forecast (TAF) for an airport
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn taf(
    ctx: Context<'_>,
    #[description = "ICAO airport code, e.g. EPWA"] airport: String,
) -> Result<(), Error> {
    let result = fetch_taf(&ctx.data().http_client, AVIATION_WEATHER_API, &airport).await;
    reply(ctx, "TAF", result).await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Taf {
    icao_id: String,
    name: Option<String>,
    #[serde(rename = "rawTAF")]
    raw_taf: String,
}

/// The current TAF for `airport` as an embed title and description.
pub async fn fetch_taf(
    client: &reqwest::Client,
    base_url: &str,
    airport: &str,
) -> Result<(String, String), String> {
    let code = icao(airport)?;
    let forecasts: Vec<Taf> = get_json(
        client,
        &format!("{base_url}/api/data/taf"),
        &[("ids", &code), ("format", "json")],
        "aviationweather.gov",
    )
    .await?
    .unwrap_or_default();

    let taf = forecasts
        .into_iter()
        .next()
        .ok_or_else(|| format!("No TAF found for {code}!"))?;
    let title = match &taf.name {
        Some(name) => format!("{} · {}", taf.icao_id, name),
        None => taf.icao_id.clone(),
    };
    Ok((title, format!("```\n{}\n```", split_groups(&taf.raw_taf))))
}

/// One line per change group (FMddhhmm, BECMG, TEMPO, PROBnn), as TAFs are usually printed.
fn split_groups(raw: &str) -> String {
    let mut out = String::new();
    let mut previous = "";
    for word in raw.split_whitespace() {
        let is_from = word.len() > 2
            && word.starts_with("FM")
            && word[2..].chars().all(|c| c.is_ascii_digit());
        // "PROB30 TEMPO" is one group.
        let starts_group = is_from
            || word == "BECMG"
            || word.starts_with("PROB")
            || (word == "TEMPO" && !previous.starts_with("PROB"));
        if !out.is_empty() {
            out.push_str(if starts_group { "\n  " } else { " " });
        }
        out.push_str(word);
        previous = word;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::split_groups;

    #[test]
    fn change_groups_start_new_lines() {
        let raw = "TAF EPWA 212330Z 2200/2224 25012KT 9999 BKN012 TEMPO 2200/2211 27015G28KT \
                   PROB30 TEMPO 2202/2206 1500 SHRASN BECMG 2212/2214 30010KT FM221800 31008KT CAVOK";
        assert_eq!(
            split_groups(raw),
            "TAF EPWA 212330Z 2200/2224 25012KT 9999 BKN012\n  \
             TEMPO 2200/2211 27015G28KT\n  \
             PROB30 TEMPO 2202/2206 1500 SHRASN\n  \
             BECMG 2212/2214 30010KT\n  \
             FM221800 31008KT CAVOK"
        );
    }
}
