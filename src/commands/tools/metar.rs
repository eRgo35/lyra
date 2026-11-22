use serde::Deserialize;
use serde_json::Value;

use super::lookup::{get_json, reply};
use crate::{Context, Error};

/// Shows the current weather report (METAR) for an airport
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn metar(
    ctx: Context<'_>,
    #[description = "ICAO airport code, e.g. EPWA"] airport: String,
) -> Result<(), Error> {
    let result = fetch_metar(&ctx.data().http_client, AVIATION_WEATHER_API, &airport).await;
    reply(ctx, "METAR", result).await
}

/// aviationweather.gov (NOAA), no key needed.
pub const AVIATION_WEATHER_API: &str = "https://aviationweather.gov";

const SERVICE: &str = "aviationweather.gov";

/// Upper-cases a 4-character ICAO code, or explains what is expected.
pub(super) fn icao(airport: &str) -> Result<String, String> {
    let code = airport.trim().to_ascii_uppercase();
    if code.len() == 4 && code.chars().all(|c| c.is_ascii_alphanumeric()) {
        Ok(code)
    } else {
        Err(format!(
            "\"{}\" is not an ICAO airport code (4 characters, e.g. EPWA)!",
            airport.trim()
        ))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Metar {
    icao_id: String,
    name: Option<String>,
    raw_ob: String,
    flt_cat: Option<String>,
    temp: Option<f64>,
    dewp: Option<f64>,
    /// Degrees, or "VRB".
    wdir: Option<Value>,
    wspd: Option<f64>,
    wgst: Option<f64>,
    /// Statute miles, a number or a string like "6+".
    visib: Option<Value>,
    altim: Option<f64>,
}

/// The latest METAR for `airport` as an embed title and description.
pub async fn fetch_metar(
    client: &reqwest::Client,
    base_url: &str,
    airport: &str,
) -> Result<(String, String), String> {
    let code = icao(airport)?;
    let reports: Vec<Metar> = get_json(
        client,
        &format!("{base_url}/api/data/metar"),
        &[("ids", &code), ("format", "json")],
        SERVICE,
    )
    .await?
    .unwrap_or_default();

    let report = reports
        .into_iter()
        .next()
        .ok_or_else(|| format!("No METAR found for {code}!"))?;
    Ok(format_metar(&report))
}

fn format_metar(m: &Metar) -> (String, String) {
    let title = match &m.name {
        Some(name) => format!("{} · {}", m.icao_id, name),
        None => m.icao_id.clone(),
    };

    let mut lines = vec![format!("```{}```", m.raw_ob)];
    if let Some(category) = &m.flt_cat {
        lines.push(format!("**Flight category:** {category}"));
    }
    if let Some(speed) = m.wspd {
        let direction = match &m.wdir {
            Some(Value::Number(n)) => format!("{n}°"),
            Some(Value::String(s)) if s == "VRB" => "variable".to_string(),
            _ => "?".to_string(),
        };
        let mut wind = if speed == 0.0 {
            "calm".to_string()
        } else {
            format!("{direction} at {speed} kt")
        };
        if let Some(gust) = m.wgst {
            wind.push_str(&format!(", gusts {gust} kt"));
        }
        lines.push(format!("**Wind:** {wind}"));
    }
    match &m.visib {
        Some(Value::Number(n)) => lines.push(format!("**Visibility:** {n} SM")),
        Some(Value::String(s)) => lines.push(format!("**Visibility:** {s} SM")),
        _ => {}
    }
    if let Some(temp) = m.temp {
        let mut line = format!("**Temperature:** {temp} °C");
        if let Some(dew) = m.dewp {
            line.push_str(&format!(", dew point {dew} °C"));
        }
        lines.push(line);
    }
    if let Some(altim) = m.altim {
        lines.push(format!("**Pressure:** {altim} hPa"));
    }

    (title, lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::icao;

    #[test]
    fn icao_codes_are_normalised_and_validated() {
        assert_eq!(icao(" epwa "), Ok("EPWA".to_string()));
        assert_eq!(icao("K2J3"), Ok("K2J3".to_string()));
        assert!(icao("WAW").is_err());
        assert!(icao("EPWA&ids=KJFK").is_err());
    }
}
