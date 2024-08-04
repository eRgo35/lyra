use scraper::{Html, Selector};
use serde_json::Value;
use std::error::Error;

#[derive(Debug, Clone)]
pub(crate) struct SpotifyTrack {
    pub title: String,
    pub artist: String,
}

pub(crate) const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36";

pub(crate) fn parse_single(content: String) -> Result<SpotifyTrack, Box<dyn Error>> {
    let document = Html::parse_document(&content);
    let selector = Selector::parse("#__NEXT_DATA__").unwrap();

    if let Some(script_element) = document.select(&selector).next() {
        let json_str = script_element.inner_html();
        let json_value: Value = serde_json::from_str(&json_str)?;
        let metadata = &json_value["props"]["pageProps"]["state"]["data"]["entity"];

        // println!("{metadata:?}");

        let title = metadata["title"].as_str().unwrap().to_string();

        let artists = metadata["artists"].as_array().unwrap();
        let artists_list: Vec<String> = artists
            .iter()
            .map(|artist| artist["name"].as_str().unwrap().to_string())
            .collect();
        let artist: String = artists_list.join(", ").to_string();

        let track = SpotifyTrack { title, artist };

        Ok(track)
    } else {
        Err("Could not find element".into())
    }
}

pub(crate) fn parse_list(content: String) -> Result<Vec<SpotifyTrack>, Box<dyn Error>> {
    let document = Html::parse_document(&content);
    let selector = Selector::parse("#__NEXT_DATA__").unwrap();

    if let Some(script_element) = document.select(&selector).next() {
        let json_str = script_element.inner_html();
        let json_value: Value = serde_json::from_str(&json_str)?;
        let metadata = &json_value["props"]["pageProps"]["state"]["data"]["entity"]["trackList"]
            .as_array()
            .unwrap();

        // println!("{metadata:?}");

        let tracks: Vec<SpotifyTrack> = metadata
            .iter()
            .map(|track| {
                let title = track["title"].as_str().unwrap().to_string();
                let artist = track["subtitle"].as_str().unwrap().to_string();

                SpotifyTrack { title, artist }
            })
            .collect();

        // println!("{tracks:?}");

        Ok(tracks)
    } else {
        Err("Could not find element".into())
    }
}
