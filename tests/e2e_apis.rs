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
