use crate::*;

pub fn retrieve_track(category: &str, id: &str) -> Result<String, String> {
    let embed_url = format!("https://embed.spotify.com/?uri=spotify:{}:{}", category, id);

    let client = reqwest::blocking::Client::builder()
        .use_rustls_tls()
        .user_agent(USER_AGENT)
        .build()
        .unwrap();

    let response = client.get(&embed_url).send().unwrap();
    let content = response.text().unwrap();

    let parsed_content = parse_single(content).unwrap();

    Ok(format!(
        "{} - {}",
        parsed_content.artist, parsed_content.title
    ))
}

pub fn retrieve_tracks(category: &str, id: &str) -> Result<Vec<String>, String> {
    let embed_url = format!("https://embed.spotify.com/?uri=spotify:{}:{}", category, id);

    let client = reqwest::blocking::Client::builder()
        .use_rustls_tls()
        .user_agent(USER_AGENT)
        .build()
        .unwrap();

    let response = client.get(&embed_url).send().unwrap();
    let content = response.text().unwrap();

    let parsed_content = parse_list(content).unwrap();

    let tracks: Vec<String> = parsed_content
        .iter()
        .map(|track| format!("{} - {}", track.artist, track.title))
        .collect();

    Ok(tracks)
}

pub async fn retrieve_async_track(category: &str, id: &str) -> Result<String, String> {
    let embed_url = format!("https://embed.spotify.com/?uri=spotify:{}:{}", category, id);

    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .user_agent(USER_AGENT)
        .build()
        .unwrap();

    let response = client.get(&embed_url).send().await.unwrap();
    let content = response.text().await.unwrap();

    let parsed_content = parse_single(content).unwrap();

    Ok(format!(
        "{} - {}",
        parsed_content.artist, parsed_content.title
    ))
}

pub async fn retrieve_async_tracks(category: &str, id: &str) -> Result<Vec<String>, String> {
    let embed_url = format!("https://embed.spotify.com/?uri=spotify:{}:{}", category, id);

    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .user_agent(USER_AGENT)
        .build()
        .unwrap();

    let response = client.get(&embed_url).send().await.unwrap();
    let content = response.text().await.unwrap();

    let parsed_content = parse_list(content).unwrap();

    let tracks: Vec<String> = parsed_content
        .iter()
        .map(|track| format!("{} - {}", track.artist, track.title))
        .collect();

    Ok(tracks)
}
