use serde::Deserialize;

use super::lookup::{get_json, reply};
use crate::{Context, Error};

/// Shows the current weather for a place
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn weather(
    ctx: Context<'_>,
    #[description = "City or place name"]
    #[rest]
    place: String,
) -> Result<(), Error> {
    let result = fetch_weather(&ctx.data().http_client, GEOCODING_API, FORECAST_API, &place).await;
    reply(ctx, "Weather", result).await
}

/// Open-Meteo, no key needed.
pub const GEOCODING_API: &str = "https://geocoding-api.open-meteo.com";
pub const FORECAST_API: &str = "https://api.open-meteo.com";

const SERVICE: &str = "Open-Meteo";

#[derive(Deserialize)]
struct Places {
    #[serde(default)]
    results: Vec<Place>,
}

#[derive(Deserialize)]
struct Place {
    name: String,
    latitude: f64,
    longitude: f64,
    admin1: Option<String>,
    country: Option<String>,
}

#[derive(Deserialize)]
struct Forecast {
    current: Current,
}

#[derive(Deserialize)]
struct Current {
    temperature_2m: f64,
    apparent_temperature: f64,
    relative_humidity_2m: f64,
    wind_speed_10m: f64,
    weather_code: u8,
}

/// Current conditions at the best match for `place`, as an embed title and description.
pub async fn fetch_weather(
    client: &reqwest::Client,
    geocoding_url: &str,
    forecast_url: &str,
    place: &str,
) -> Result<(String, String), String> {
    let query = place.trim();
    let not_found = || format!("Could not find a place called \"{query}\"!");
    if query.is_empty() {
        return Err("Which place? e.g. `weather Warsaw`".to_string());
    }

    let places: Places = get_json(
        client,
        &format!("{geocoding_url}/v1/search"),
        &[
            ("name", query),
            ("count", "1"),
            ("language", "en"),
            ("format", "json"),
        ],
        SERVICE,
    )
    .await?
    .ok_or_else(not_found)?;
    let place = places.results.into_iter().next().ok_or_else(not_found)?;

    let (latitude, longitude) = (place.latitude.to_string(), place.longitude.to_string());
    let forecast: Forecast = get_json(
        client,
        &format!("{forecast_url}/v1/forecast"),
        &[
            ("latitude", &latitude),
            ("longitude", &longitude),
            (
                "current",
                "temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code",
            ),
        ],
        SERVICE,
    )
    .await?
    .ok_or_else(|| format!("No forecast for {}!", place.name))?;

    let title = [Some(place.name), place.admin1, place.country]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ");
    let c = forecast.current;
    let description = format!(
        "{}\n**Temperature:** {} °C (feels like {} °C)\n**Humidity:** {}%\n**Wind:** {} km/h",
        describe(c.weather_code),
        c.temperature_2m,
        c.apparent_temperature,
        c.relative_humidity_2m,
        c.wind_speed_10m,
    );
    Ok((title, description))
}

/// WMO weather interpretation code (as used by Open-Meteo) in words.
fn describe(code: u8) -> &'static str {
    match code {
        0 => "☀️ Clear sky",
        1 => "🌤️ Mainly clear",
        2 => "⛅ Partly cloudy",
        3 => "☁️ Overcast",
        45 | 48 => "🌫️ Fog",
        51 | 53 | 55 => "🌦️ Drizzle",
        56 | 57 => "🌧️ Freezing drizzle",
        61 | 63 | 65 => "🌧️ Rain",
        66 | 67 => "🌧️ Freezing rain",
        71 | 73 | 75 | 77 => "🌨️ Snow",
        80..=82 => "🌦️ Rain showers",
        85 | 86 => "🌨️ Snow showers",
        95 => "⛈️ Thunderstorm",
        96 | 99 => "⛈️ Thunderstorm with hail",
        _ => "🌡️ Unknown conditions",
    }
}

#[cfg(test)]
mod tests {
    use super::describe;

    #[test]
    fn wmo_codes_have_descriptions() {
        assert_eq!(describe(0), "☀️ Clear sky");
        assert_eq!(describe(81), "🌦️ Rain showers");
        assert_eq!(describe(99), "⛈️ Thunderstorm with hail");
        assert_eq!(describe(42), "🌡️ Unknown conditions");
    }
}
