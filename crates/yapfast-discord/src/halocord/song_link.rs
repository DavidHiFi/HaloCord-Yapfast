//! HaloCord's Song link, its buttons: where a song is on each service, as
//! song.link's own page lists it, from its public API
//! (`api.song.link/v1-alpha.1/links`). Asked only when someone presses for
//! it, once per press; HaloCord's window keeps the answers.

use std::fmt::Write as _;

use serde_json::Value;

/// One service's link for a song, from song.link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SongPlatform {
    /// The API's key for it, such as `spotify` or `appleMusic`.
    pub key: String,
    /// Its page.
    pub url: String,
    /// What its desktop app opens, such as `spotify:track:...`, when there
    /// is one.
    pub native: Option<String>,
}

/// Where the song at `link` is on each service, for `country` when that's
/// two letters.
pub async fn song_platforms(link: &str, country: &str) -> Result<Vec<SongPlatform>, String> {
    static HTTP: std::sync::OnceLock<Option<reqwest::Client>> = std::sync::OnceLock::new();
    let http = HTTP
        .get_or_init(|| {
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
            reqwest::Client::builder()
                .user_agent("Yapfast (HaloCord Song link)")
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(20))
                .build()
                .ok()
        })
        .as_ref()
        .ok_or("no HTTPS client")?;
    let response = http
        .get(api_address(link, country))
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("song.link answered {}", response.status()));
    }
    let body = response.bytes().await.map_err(|error| error.to_string())?;
    let answer: Value = serde_json::from_slice(&body).map_err(|error| error.to_string())?;
    Ok(song_platforms_in(&answer))
}

/// The API's address asking for `link`, which goes in encoded.
fn api_address(link: &str, country: &str) -> String {
    let mut address = String::from("https://api.song.link/v1-alpha.1/links?url=");
    for byte in link.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            address.push(char::from(byte));
        } else {
            let _ = write!(address, "%{byte:02X}");
        }
    }
    let country = country.trim();
    if country.len() == 2 && country.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        let _ = write!(address, "&userCountry={}", country.to_uppercase());
    }
    address
}

/// The services of song.link's answer, in its order; only pages on HTTPS.
pub fn song_platforms_in(answer: &Value) -> Vec<SongPlatform> {
    answer["linksByPlatform"]
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(key, link)| {
            let url = link["url"]
                .as_str()
                .filter(|url| url.starts_with("https://"))?;
            Some(SongPlatform {
                key: key.clone(),
                url: url.to_owned(),
                native: link["nativeAppUriDesktop"].as_str().map(str::to_owned),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn reads_each_services_link_and_asks_with_the_link_encoded() {
        let found = song_platforms_in(&json!({"linksByPlatform": {
            "spotify": {"url": "https://open.spotify.com/track/1", "nativeAppUriDesktop": "spotify:track:1"},
            "deezer": {"url": "https://www.deezer.com/track/2"},
            "odd": {"url": "javascript:x"},
        }}));
        assert_eq!(found.len(), 2, "not a page on HTTPS: left out");
        let spotify = found.iter().find(|found| found.key == "spotify").unwrap();
        assert_eq!(spotify.native.as_deref(), Some("spotify:track:1"));
        assert_eq!(
            api_address("https://music.youtube.com/watch?v=a&list=b", ""),
            "https://api.song.link/v1-alpha.1/links?url=https%3A%2F%2Fmusic.youtube.com%2Fwatch%3Fv%3Da%26list%3Db"
        );
        assert!(api_address("https://tidal.com/track/1", "gb").ends_with("&userCountry=GB"));
    }

    #[test]
    #[ignore = "asks api.song.link"]
    fn song_link_lists_a_song() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let found = runtime
            .block_on(song_platforms(
                "https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC",
                "",
            ))
            .unwrap();
        assert!(found.iter().any(|found| found.key == "spotify"));
    }
}
