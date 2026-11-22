//! `metar`, `taf`, `weather`, `whois` and `ip` against a local mock server.

use lyra::commands::tools::{ip::fetch_ip, metar::fetch_metar, taf::fetch_taf};
use lyra::commands::tools::{weather::fetch_weather, whois::fetch_domain};
use wiremock::matchers::{header_regex, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn json(status: u16, body: &str) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_raw(body.to_owned(), "application/json")
}

async fn serve(route: &str, response: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(route))
        .respond_with(response)
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn metar_is_decoded_and_the_code_normalised() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/data/metar"))
        .and(query_param("ids", "EPWA"))
        .and(query_param("format", "json"))
        .respond_with(json(
            200,
            r#"[{"icaoId": "EPWA", "name": "Warsaw/Chopin Arpt", "fltCat": "VFR",
                 "rawOb": "METAR EPWA 220030Z 28015G25KT 9999 SCT016 04/01 Q1007",
                 "temp": 4, "dewp": 1, "wdir": 280, "wspd": 15, "wgst": 25,
                 "visib": "6+", "altim": 1007}]"#,
        ))
        .expect(1)
        .mount(&server)
        .await;

    let (title, description) = fetch_metar(&reqwest::Client::new(), &server.uri(), "epwa")
        .await
        .unwrap();
    assert_eq!(title, "EPWA · Warsaw/Chopin Arpt");
    assert_eq!(
        description,
        "```METAR EPWA 220030Z 28015G25KT 9999 SCT016 04/01 Q1007```\n\
         **Flight category:** VFR\n\
         **Wind:** 280° at 15 kt, gusts 25 kt\n\
         **Visibility:** 6+ SM\n\
         **Temperature:** 4 °C, dew point 1 °C\n\
         **Pressure:** 1007 hPa"
    );
}

#[tokio::test]
async fn metar_errors() {
    let client = reqwest::Client::new();
    for response in [ResponseTemplate::new(204), json(200, "[]")] {
        let server = serve("/api/data/metar", response).await;
        assert_eq!(
            fetch_metar(&client, &server.uri(), "ZZZZ").await,
            Err("No METAR found for ZZZZ!".to_string())
        );
    }
    let server = serve("/api/data/metar", json(502, "")).await;
    assert_eq!(
        fetch_metar(&client, &server.uri(), "EPWA").await,
        Err("Got an error from aviationweather.gov (502 Bad Gateway)!".to_string())
    );
    // Invalid codes never reach the network.
    assert!(fetch_metar(&client, "http://127.0.0.1:9", "WAW")
        .await
        .unwrap_err()
        .contains("not an ICAO airport code"));
}

#[tokio::test]
async fn taf_is_split_into_change_groups() {
    let server = serve(
        "/api/data/taf",
        json(
            200,
            r#"[{"icaoId": "EPWA", "name": "Warsaw/Chopin Arpt",
                 "rawTAF": "TAF EPWA 212330Z 2200/2224 25012KT 9999 TEMPO 2200/2211 4000 SHRA "}]"#,
        ),
    )
    .await;
    let (title, description) = fetch_taf(&reqwest::Client::new(), &server.uri(), "EPWA")
        .await
        .unwrap();
    assert_eq!(title, "EPWA · Warsaw/Chopin Arpt");
    assert_eq!(
        description,
        "```\nTAF EPWA 212330Z 2200/2224 25012KT 9999\n  TEMPO 2200/2211 4000 SHRA\n```"
    );
}

#[tokio::test]
async fn weather_geocodes_then_fetches_current_conditions() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/search"))
        .and(query_param("name", "New York"))
        .respond_with(json(
            200,
            r#"{"results": [{"name": "New York", "latitude": 40.71, "longitude": -74.01,
                              "admin1": "New York", "country": "United States"}]}"#,
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/forecast"))
        .and(query_param("latitude", "40.71"))
        .and(query_param("longitude", "-74.01"))
        .respond_with(json(
            200,
            r#"{"current": {"temperature_2m": 3.9, "apparent_temperature": -1.0,
                            "relative_humidity_2m": 88, "wind_speed_10m": 21.2,
                            "weather_code": 3}}"#,
        ))
        .expect(1)
        .mount(&server)
        .await;

    let (title, description) = fetch_weather(
        &reqwest::Client::new(),
        &server.uri(),
        &server.uri(),
        " New York ",
    )
    .await
    .unwrap();
    assert_eq!(title, "New York, New York, United States");
    assert_eq!(
        description,
        "☁️ Overcast\n**Temperature:** 3.9 °C (feels like -1 °C)\n**Humidity:** 88%\n**Wind:** 21.2 km/h"
    );
}

#[tokio::test]
async fn weather_reports_unknown_places() {
    let server = serve("/v1/search", json(200, r#"{"generationtime_ms": 0.1}"#)).await;
    assert_eq!(
        fetch_weather(
            &reqwest::Client::new(),
            &server.uri(),
            &server.uri(),
            "zzqqxx"
        )
        .await,
        Err("Could not find a place called \"zzqqxx\"!".to_string())
    );
}

#[tokio::test]
async fn whois_summarises_rdap_domain_data() {
    let server = serve(
        "/domain/example.com",
        json(
            200,
            r#"{"ldhName": "EXAMPLE.COM",
                "status": ["client transfer prohibited"],
                "events": [{"eventAction": "registration", "eventDate": "1995-08-14T04:00:00Z"},
                           {"eventAction": "expiration", "eventDate": "2027-08-13T04:00:00Z"}],
                "entities": [{"roles": ["registrar"],
                              "vcardArray": ["vcard", [["version", {}, "text", "4.0"],
                                                       ["fn", {}, "text", "IANA"]]]}],
                "nameservers": [{"ldhName": "A.IANA-SERVERS.NET"}]}"#,
        ),
    )
    .await;
    let (title, description) = fetch_domain(&reqwest::Client::new(), &server.uri(), "Example.com")
        .await
        .unwrap();
    assert_eq!(title, "example.com");
    assert_eq!(
        description,
        "**Registrar:** IANA\n**Registered:** 1995-08-14\n**Expires:** 2027-08-13\n\
         **Status:** client transfer prohibited\n**Name servers:** a.iana-servers.net"
    );

    let server = serve("/domain/nope.com", json(404, "{}")).await;
    assert_eq!(
        fetch_domain(&reqwest::Client::new(), &server.uri(), "nope.com").await,
        Err("No registration data found for nope.com!".to_string())
    );
}

#[tokio::test]
async fn ip_summarises_rdap_network_data() {
    let server = serve(
        "/ip/1.1.1.1",
        json(
            200,
            r#"{"name": "APNIC-LABS", "startAddress": "1.1.1.0", "endAddress": "1.1.1.255",
                "country": "AU",
                "entities": [
                  {"roles": ["registrant"],
                   "vcardArray": ["vcard", [["fn", {}, "text", "APNIC Research and Development"]]],
                   "entities": [{"roles": ["abuse"],
                                 "vcardArray": ["vcard", [["email", {}, "text", "helpdesk@apnic.net"]]]}]}
                ]}"#,
        ),
    )
    .await;
    let (title, description) = fetch_ip(
        &reqwest::Client::new(),
        &server.uri(),
        "1.1.1.1".parse().unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(title, "1.1.1.1");
    assert_eq!(
        description,
        "**Network:** APNIC-LABS\n**Range:** 1.1.1.0 – 1.1.1.255\n**Country:** AU\n\
         **Owner:** APNIC Research and Development\n**Abuse contact:** helpdesk@apnic.net"
    );
}

#[tokio::test]
async fn lookups_send_a_user_agent() {
    // rdap.org answers 403 to requests without one.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/domain/example.com"))
        .and(header_regex("user-agent", r"^lyra/\d+\.\d+\.\d+ "))
        .respond_with(json(200, r#"{"ldhName": "EXAMPLE.COM"}"#))
        .expect(1)
        .mount(&server)
        .await;
    assert!(
        fetch_domain(&reqwest::Client::new(), &server.uri(), "example.com")
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn lyrics_pick_the_first_result_with_lyrics() {
    use lyra::commands::music::lyrics::fetch_lyrics;
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .and(query_param("q", "rick astley never gonna"))
        .respond_with(json(
            200,
            r#"[{"trackName": "Never Gonna (Karaoke)", "artistName": "Nobody", "plainLyrics": null},
                {"trackName": "Never Gonna Give You Up", "artistName": "Rick Astley",
                 "instrumental": false, "plainLyrics": "We're no strangers to love\n"}]"#,
        ))
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(
        fetch_lyrics(
            &reqwest::Client::new(),
            &server.uri(),
            "rick astley never gonna"
        )
        .await,
        Ok((
            "Rick Astley – Never Gonna Give You Up".to_string(),
            "We're no strangers to love".to_string()
        ))
    );

    let server = serve("/api/search", json(200, "[]")).await;
    assert_eq!(
        fetch_lyrics(&reqwest::Client::new(), &server.uri(), "zzqq").await,
        Err("No lyrics found for \"zzqq\"!".to_string())
    );
}
