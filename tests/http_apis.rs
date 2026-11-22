//! `dictionary` and `verse` against a local mock server: success, the error paths, and
//! that user input is URL-encoded into the request path.

use lyra::commands::tools::dictionary::define;
use lyra::commands::tools::verse::fetch_verse;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const HELLO: &str = r#"[{
    "word": "hello",
    "phonetics": [{"audio": ""}, {"text": "/həˈləʊ/"}],
    "meanings": [
        {"partOfSpeech": "noun", "definitions": [{"definition": "A greeting."}]},
        {"partOfSpeech": "verb", "definitions": [{"definition": "To greet."}]}
    ]
}]"#;

async fn serve(route: &str, response: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(route))
        .respond_with(response)
        .expect(1)
        .mount(&server)
        .await;
    server
}

fn json(status: u16, body: &str) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_raw(body.to_owned(), "application/json")
}

/// A base URL nothing listens on.
async fn closed_port() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    format!("http://{addr}")
}

#[tokio::test]
async fn dictionary_formats_a_found_word() {
    let server = serve("/hello", json(200, HELLO)).await;

    let (title, description) = define(&reqwest::Client::new(), &server.uri(), "hello")
        .await
        .unwrap();

    assert_eq!(title, "hello /həˈləʊ/");
    assert_eq!(description, "**noun**: A greeting.\n**verb**: To greet.");
}

#[tokio::test]
async fn dictionary_encodes_the_word_into_one_path_segment() {
    let server = serve("/ice+cream%2F%3F", json(200, HELLO)).await;

    assert!(
        define(&reqwest::Client::new(), &server.uri(), "ice cream/?")
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn dictionary_error_messages() {
    let client = reqwest::Client::new();
    let cases = [
        (
            json(404, r#"{"title": "No Definitions Found"}"#),
            "No definitions found for \"qwzx\"!",
        ),
        (json(200, "[]"), "No definitions found for \"qwzx\"!"),
        (
            json(200, "not json"),
            "Could not read the dictionary response!",
        ),
        (
            json(500, "{}"),
            "The dictionary returned an error (500 Internal Server Error)!",
        ),
        (
            json(429, "{}"),
            "The dictionary returned an error (429 Too Many Requests)!",
        ),
    ];

    for (response, expected) in cases {
        let server = serve("/qwzx", response).await;
        assert_eq!(
            define(&client, &server.uri(), "qwzx").await,
            Err(expected.to_string())
        );
    }

    assert_eq!(
        define(&client, &closed_port().await, "qwzx").await,
        Err("Could not reach the dictionary!".to_string())
    );
}

#[tokio::test]
async fn verse_returns_the_quote_and_picks_the_translation() {
    let client = reqwest::Client::new();
    for (latin, translation) in [(false, "web"), (true, "clementine")] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/john+3%3A16"))
            .and(query_param("translation", translation))
            .respond_with(json(
                200,
                r#"{"reference": "John 3:16", "text": "For God so loved the world…",
                    "translation_name": "Test Bible", "translation_note": "", "verses": []}"#,
            ))
            .expect(1)
            .mount(&server)
            .await;

        let found = fetch_verse(&client, &server.uri(), "john 3:16", latin)
            .await
            .unwrap();
        assert_eq!(found.reference, "John 3:16");
        assert_eq!(found.translation_name, "Test Bible");
    }
}

#[tokio::test]
async fn verse_error_messages() {
    let client = reqwest::Client::new();
    let long = format!(
        r#"{{"reference": "Psalm 119", "text": "{}", "translation_name": "T"}}"#,
        "a".repeat(4001)
    );
    let cases = [
        (
            json(404, r#"{"error": "not found"}"#),
            "Verse \"nope 9:9\" not found!",
        ),
        (json(200, &long), "Quoted text is too long!"),
        (
            json(200, r#"{"text": 1}"#),
            "Could not read the Bible API response!",
        ),
        (
            json(503, "{}"),
            "The Bible API returned an error (503 Service Unavailable)!",
        ),
    ];

    for (response, expected) in cases {
        let server = serve("/nope+9%3A9", response).await;
        assert_eq!(
            fetch_verse(&client, &server.uri(), "nope 9:9", false)
                .await
                .map(|v| v.text),
            Err(expected.to_string())
        );
    }

    assert_eq!(
        fetch_verse(&client, &closed_port().await, "nope 9:9", false)
            .await
            .map(|v| v.text),
        Err("Could not reach the Bible API!".to_string())
    );
}
