mod error;
mod parser;
mod retriever;

use error::ParseError;
use parser::{parse_list, parse_single, USER_AGENT};
use regex::Regex;
use reqwest;
use retriever::{retrieve_async_track, retrieve_async_tracks, retrieve_track, retrieve_tracks};

pub fn retrieve_url(url: &str) -> Result<Vec<String>, ParseError> {
    let spotify_regex: Regex = Regex::new(r"https?:\/\/(?:embed\.|open\.)spotify\.com\/(track|album|playlist)\/([a-zA-Z0-9]+)(?:\?si=[\w-]+)?").unwrap();

    if let Some(captures) = spotify_regex.captures(url) {
        let category = captures.get(1).unwrap().as_str();
        let id = captures.get(2).unwrap().as_str();

        match category {
            "track" => {
                let track = retrieve_track(category, id).unwrap();
                Ok(vec![track])
            }
            "playlist" | "album" => {
                let tracks = retrieve_tracks(category, id).unwrap();
                Ok(tracks)
            }
            _ => Err(ParseError::InvalidUrl),
        }
    } else {
        Err(ParseError::InvalidUrl)
    }
}

pub async fn retrieve_async_url(url: &str) -> Result<Vec<String>, ParseError> {
    let spotify_regex: Regex = Regex::new(r"https?:\/\/(?:embed\.|open\.)spotify\.com\/(track|album|playlist)\/([a-zA-Z0-9]+)(?:\?si=[\w-]+)?").unwrap();

    if let Some(captures) = spotify_regex.captures(url) {
        let category = captures.get(1).unwrap().as_str();
        let id = captures.get(2).unwrap().as_str();

        match category {
            "track" => {
                let track = retrieve_async_track(category, id).await.unwrap();
                Ok(vec![track])
            }
            "playlist" | "album" => {
                let tracks = retrieve_async_tracks(category, id).await.unwrap();
                Ok(tracks)
            }
            _ => Err(ParseError::InvalidUrl),
        }
    } else {
        Err(ParseError::InvalidUrl)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRACK: &str = "https://open.spotify.com/track/4PTG3Z6ehGkBFwjybzWkR8?si=e0a8c8ada8284e43";
    const MULTIPLE_ARTISTS_TRACK: &str =
        "https://open.spotify.com/track/1O0SdrryPeGp6eSSUibdgo?si=4ae58febe9e74eae";
    const PLAYLIST: &str =
        "https://open.spotify.com/playlist/37i9dQZF1DZ06evO05tE88?si=e0c6f44d176f44e6";
    const ALBUM: &str =
        "https://open.spotify.com/album/6eUW0wxWtzkFdaEFsTJto6?si=_grLtlNySNyfJTZr8tP44Q";

    #[test]
    fn check_track() {
        let track: Vec<&str> = vec!["Rick Astley - Never Gonna Give You Up"];
        assert_eq!(retrieve_url(TRACK).unwrap(), track);
    }

    #[test]
    fn check_multiple_artists_track() {
        let track: Vec<&str> = vec!["Will o' the wisp, Rick Astley - Blood on My Tie"];
        assert_eq!(retrieve_url(MULTIPLE_ARTISTS_TRACK).unwrap(), track);
    }

    #[test]
    fn check_playlist() {
        let playlist: Vec<&str> = vec![
            "Rick Astley - Never Gonna Give You Up",
            "Rick Astley - Take Me to Your Heart (2023 Remaster)",
            "Rick Astley - Cry for Help - Single Edit",
            "Rick Astley - Never Gonna Stop",
            "Rick Astley - Together Forever",
            "Rick Astley - Hold Me in Your Arms (7\" Version)",
            "Rick Astley - Angels on My Side",
            "New Kids On The Block, Salt-N-Pepa, Rick Astley, En Vogue - Bring Back The Time",
            "Rick Astley - Whenever You Need Somebody",
            "Rick Astley - She Wants to Dance with Me (2023 Remaster)",
            "Rick Astley - Dippin My Feet",
            "Rick Astley - My Arms Keep Missing You",
            "Rick Astley - Don't Say Goodbye",
            "Rick Astley - Dance",
            "Rick Astley - Giving Up On Love (7'' Pop Version)",
            "Rick Astley - Never Gonna Give You Up (Cake Mix)",
            "Rick Astley - It Would Take a Strong Strong Man",
            "Rick Astley - Driving Me Crazy",
            "Rick Astley - Beautiful Life",
            "Rick Astley - I Don't Want to Lose Her",
            "Rick Astley - Keep Singing",
            "Rick Astley - Forever and More",
            "Rick Astley - Hopelessly",
            "Rick Astley - When I Fall in Love",
            "Rick Astley - Every One of Us",
            "Rick Astley - High Enough",
            "Rick Astley - Ain't Too Proud to Beg (2023 Remaster)",
            "Rick Astley - I'll Never Let You Down",
            "Trevor Horn, Rick Astley - Owner Of A Lonely Heart",
            "Rick Astley - Letting Go",
            "Rick Astley - Never Knew Love",
            "Rick Astley - Lights Out - Radio Edit",
            "Rick Astley - Try",
            "Rick Astley - Dial My Number (2023 Remaster)",
            "Rick Astley - Wish Away",
            "Rick Astley - Giant",
            "Rick Astley - Move Right Out",
            "Rick Astley - Till Then (Time Stands Still) (2023 Remaster)",
            "Rick Astley - Pray with Me",
            "Rick Astley - I Don't Want to Be Your Lover",
            "Will o' the wisp, Rick Astley - Blood on My Tie",
            "Rick Astley - Can't Help Falling in Love",
            "Rick Astley - I Like the Sun",
            "Rick Astley - She Makes Me",
            "Rick Astley - Body and Soul",
            "Rick Astley - (They Long to Be) Close to You",
            "Rick Astley - Unwanted (Official Song from the Podcast)",
            "Rick Astley - Last Night on Earth",
            "Rick Astley - Everlong - Acoustic Version",
            "Rick Astley - Superman",
        ];
        assert_eq!(retrieve_url(PLAYLIST).unwrap(), playlist);
    }

    #[test]
    fn check_album() {
        let album: Vec<&str> = vec![
            "Rick Astley - Never Gonna Give You Up",
            "Rick Astley - Whenever You Need Somebody",
            "Rick Astley - Together Forever",
            "Rick Astley - It Would Take a Strong Strong Man",
            "Rick Astley - The Love Has Gone",
            "Rick Astley - Don't Say Goodbye",
            "Rick Astley - Slipping Away",
            "Rick Astley - No More Looking for Love",
            "Rick Astley - You Move Me",
            "Rick Astley - When I Fall in Love",
        ];
        assert_eq!(retrieve_url(ALBUM).unwrap(), album);
    }
}
