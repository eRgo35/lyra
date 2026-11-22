//! End-to-end: `dictionary` and `verse` against the real public APIs.
//!
//! Needs network access, so these are ignored by default:
//! `cargo test --test e2e_apis -- --ignored`

use lyra::commands::tools::dictionary::{define, DICTIONARY_API};
use lyra::commands::tools::verse::{fetch_verse, BIBLE_API};

#[tokio::test]
#[ignore = "network: api.dictionaryapi.dev"]
async fn dictionary_defines_a_real_word() {
    let (title, description) = define(&reqwest::Client::new(), DICTIONARY_API, "hello")
        .await
        .unwrap();

    assert!(title.starts_with("hello"), "{title}");
    assert!(description.contains("**noun**"), "{description}");
}

#[tokio::test]
#[ignore = "network: api.dictionaryapi.dev"]
async fn dictionary_reports_unknown_words() {
    assert_eq!(
        define(&reqwest::Client::new(), DICTIONARY_API, "qwzxqv").await,
        Err("No definitions found for \"qwzxqv\"!".to_string())
    );
}

#[tokio::test]
#[ignore = "network: bible-api.com"]
async fn verse_in_english_and_latin() {
    let client = reqwest::Client::new();

    let english = fetch_verse(&client, BIBLE_API, "john 3:16", false)
        .await
        .unwrap();
    assert_eq!(english.reference, "John 3:16");
    assert!(english.text.contains("God"), "{}", english.text);

    let latin = fetch_verse(&client, BIBLE_API, "john 3:16", true)
        .await
        .unwrap();
    assert!(latin.text.contains("Deus"), "{}", latin.text);
}

#[tokio::test]
#[ignore = "network: bible-api.com"]
async fn verse_reports_unknown_references() {
    assert_eq!(
        fetch_verse(&reqwest::Client::new(), BIBLE_API, "notabook 99:99", false)
            .await
            .map(|v| v.text),
        Err("Verse \"notabook 99:99\" not found!".to_string())
    );
}

#[tokio::test]
#[ignore = "network: aviationweather.gov"]
async fn metar_and_taf_for_a_real_airport() {
    use lyra::commands::tools::metar::{fetch_metar, AVIATION_WEATHER_API};
    use lyra::commands::tools::taf::fetch_taf;
    let client = reqwest::Client::new();

    let (title, description) = fetch_metar(&client, AVIATION_WEATHER_API, "KJFK")
        .await
        .unwrap();
    assert!(title.starts_with("KJFK"), "{title}");
    assert!(description.contains("KJFK"), "{description}");

    let (_, description) = fetch_taf(&client, AVIATION_WEATHER_API, "KJFK")
        .await
        .unwrap();
    assert!(description.contains("TAF"), "{description}");
}

#[tokio::test]
#[ignore = "network: open-meteo.com"]
async fn weather_for_a_real_city() {
    use lyra::commands::tools::weather::{fetch_weather, FORECAST_API, GEOCODING_API};
    let (title, description) = fetch_weather(
        &reqwest::Client::new(),
        GEOCODING_API,
        FORECAST_API,
        "Warsaw",
    )
    .await
    .unwrap();
    assert!(title.contains("Poland"), "{title}");
    assert!(description.contains("°C"), "{description}");
}

#[tokio::test]
#[ignore = "network: rdap.org"]
async fn whois_and_ip_via_rdap() {
    use lyra::commands::tools::ip::{fetch_ip, resolve};
    use lyra::commands::tools::whois::{fetch_domain, RDAP_API};
    let client = reqwest::Client::new();

    let (_, description) = fetch_domain(&client, RDAP_API, "example.com")
        .await
        .unwrap();
    assert!(
        description.contains("**Registered:** 1995-08-14"),
        "{description}"
    );

    let address = resolve("one.one.one.one").await.unwrap();
    let (_, description) = fetch_ip(&client, RDAP_API, address).await.unwrap();
    assert!(description.contains("APNIC"), "{description}");
}

#[tokio::test]
#[ignore = "network: lrclib.net"]
async fn lyrics_for_a_real_song() {
    use lyra::commands::music::lyrics::{fetch_lyrics, LRCLIB_API};
    let (title, lyrics) = fetch_lyrics(
        &reqwest::Client::new(),
        LRCLIB_API,
        "Rick Astley - Never Gonna Give You Up",
    )
    .await
    .unwrap();
    assert!(title.contains("Never Gonna Give You Up"), "{title}");
    assert!(lyrics.contains("strangers to love"), "{lyrics}");
}
