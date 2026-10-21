//! Experimental Music integration. A successful live authenticated track is a
//! release gate. There are no alternate clients, cookie imports, token providers,
//! signature/cipher transformations, advertisement skipping, or entitlement overrides.
use super::{Account, Value};
use kyuutube_audio_spike::RemoteMedia;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;

#[derive(Clone, Serialize, Deserialize)]
pub struct MusicTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
}
pub struct MusicStream {
    pub track: MusicTrack,
    pub media: RemoteMedia,
}
fn valid_id(id: &str) -> bool {
    id.len() == 11
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}
fn context() -> Value {
    let now = time::OffsetDateTime::now_utc();
    json!({"client":{"clientName":"WEB_REMIX", "clientVersion":format!("1.{:04}{:02}{:02}.01.00", now.year(), u8::from(now.month()), now.day()), "hl":"en"}, "user":{}})
}
impl Account {
    fn music_request(&self, endpoint: &str, mut body: Value) -> Result<Value, String> {
        body["context"] = context();
        let result = self.http.music(endpoint, &self.token()?, &body);
        if result
            .as_ref()
            .is_err_and(|error| error.starts_with("YouTube rejected authorization"))
        {
            *self
                .access
                .lock()
                .map_err(|_| "Account state unavailable")? = None;
            self.http.music(endpoint, &self.token()?, &body)
        } else {
            result
        }
    }
    pub(super) fn verify_music_account(&self) -> Result<(), String> {
        let value = self.music_request("account/account_menu", json!({}))?;
        // An anonymous account-menu response is not proof of Music authorization.
        if !has_account_header(&value, 0) {
            return Err("YouTube Music did not verify this account".into());
        }
        Ok(())
    }
    pub(super) fn verify_music_with_token(&self, token: &str) -> Result<(), String> {
        let value =
            self.http
                .music("account/account_menu", token, &json!({"context":context()}))?;
        if !has_account_header(&value, 0) {
            return Err("YouTube Music did not verify this account".into());
        }
        Ok(())
    }
    pub fn search_music(&self, query: String) -> Result<Vec<MusicTrack>, String> {
        let _operation = self
            .operation
            .try_lock()
            .map_err(|_| "An account operation is already running")?;
        let query = query.trim();
        if query.is_empty() || query.len() > 200 {
            return Err("Enter a shorter music search".into());
        }
        self.verify_music_account()?;
        let value = self.music_request("search", json!({"query": query}))?;
        let mut tracks = Vec::new();
        collect_tracks(&value, 0, &mut HashSet::new(), &mut tracks);
        Ok(tracks)
    }
    pub fn resolve_music_stream(&self, id: String) -> Result<MusicStream, String> {
        let _operation = self
            .operation
            .try_lock()
            .map_err(|_| "An account operation is already running")?;
        if !valid_id(&id) {
            return Err("Invalid track identifier".into());
        }
        self.verify_music_account()?;
        let value = self.music_request("player", json!({"videoId":id}))?;
        parse_stream(value, &id)
    }
}
fn has_account_header(value: &Value, depth: u8) -> bool {
    if depth > 24 {
        return false;
    }
    if let Some(header) = value.get("activeAccountHeaderRenderer") {
        if !runs(&header["accountName"]).is_empty() {
            return true;
        }
    }
    match value {
        Value::Object(map) => map
            .values()
            .any(|value| has_account_header(value, depth + 1)),
        Value::Array(items) => items
            .iter()
            .any(|value| has_account_header(value, depth + 1)),
        _ => false,
    }
}
fn runs(value: &Value) -> String {
    if let Some(simple) = value["simpleText"].as_str() {
        return simple.chars().take(512).collect();
    }
    value["runs"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .take(32)
                .filter_map(|item| item["text"].as_str())
                .collect::<String>()
                .chars()
                .take(512)
                .collect()
        })
        .unwrap_or_default()
}
fn collect_tracks(
    value: &Value,
    depth: u8,
    seen: &mut HashSet<String>,
    tracks: &mut Vec<MusicTrack>,
) {
    if depth > 24 || tracks.len() >= 25 {
        return;
    }
    for key in ["musicResponsiveListItemRenderer", "musicTwoRowItemRenderer"] {
        if let Some(renderer) = value.get(key) {
            let first =
                &renderer["flexColumns"][0]["musicResponsiveListItemFlexColumnRenderer"]["text"];
            let id = renderer["playlistItemData"]["videoId"]
                .as_str()
                .or_else(|| renderer["navigationEndpoint"]["watchEndpoint"]["videoId"].as_str())
                .or_else(|| {
                    first["runs"][0]["navigationEndpoint"]["watchEndpoint"]["videoId"].as_str()
                });
            if let Some(id) = id.filter(|id| valid_id(id)) {
                let title = if renderer["title"].is_object() {
                    runs(&renderer["title"])
                } else {
                    runs(first)
                };
                if !title.is_empty() && seen.insert(id.to_owned()) {
                    let artist = if renderer["subtitle"].is_object() {
                        runs(&renderer["subtitle"])
                    } else {
                        runs(
                            &renderer["flexColumns"][1]
                                ["musicResponsiveListItemFlexColumnRenderer"]["text"],
                        )
                    };
                    tracks.push(MusicTrack {
                        id: id.into(),
                        title,
                        artist,
                    });
                }
            }
        }
    }
    match value {
        Value::Object(map) => {
            for value in map.values() {
                collect_tracks(value, depth + 1, seen, tracks);
            }
        }
        Value::Array(items) => {
            for value in items {
                collect_tracks(value, depth + 1, seen, tracks);
            }
        }
        _ => (),
    }
}
fn parse_stream(value: Value, id: &str) -> Result<MusicStream, String> {
    if value["playabilityStatus"]["status"] != "OK" {
        return Err("YouTube Music does not allow playback of this track".into());
    }
    // This cannot certify the absence of ads for an account type. Live account
    // and advertisement behavior must still be verified before publication.
    for key in ["adPlacements", "playerAds", "adSlots"] {
        if value.get(key).is_some_and(|value| {
            !value.is_null() && value.as_array().is_none_or(|items| !items.is_empty())
        }) {
            return Err("This track requires ads that the native player cannot handle yet".into());
        }
    }
    if value["streamingData"]
        .get("licenseInfos")
        .is_some_and(|value| !value.is_null())
    {
        return Err("This track requires an unsupported protected player".into());
    }
    if value["videoDetails"]["videoId"] != id {
        return Err("YouTube returned a different track".into());
    }
    let title = value["videoDetails"]["title"]
        .as_str()
        .filter(|title| !title.is_empty() && title.len() <= 8192)
        .ok_or("Track information is missing")?;
    let artist = value["videoDetails"]["author"]
        .as_str()
        .filter(|artist| artist.len() <= 8192)
        .unwrap_or_default();
    let formats = value["streamingData"]["adaptiveFormats"]
        .as_array()
        .ok_or("No native audio stream was returned")?;
    if formats.len() > 256 {
        return Err("Too many media formats returned".into());
    }
    let selected = formats
        .iter()
        .filter(|format| {
            format["mimeType"]
                .as_str()
                .is_some_and(|mime| mime.starts_with("audio/"))
                && format.get("signatureCipher").is_none()
                && format.get("cipher").is_none()
                && format["bitrate"]
                    .as_u64()
                    .is_some_and(|bitrate| bitrate > 0 && bitrate <= 512_000)
                && format["url"].is_string()
        })
        .max_by_key(|format| format["bitrate"].as_u64().unwrap_or(0))
        .ok_or("No supported native audio stream was returned")?;
    let media = RemoteMedia::google_audio(
        selected["url"]
            .as_str()
            .ok_or("No media address returned")?
            .to_owned(),
    )?;
    Ok(MusicStream {
        track: MusicTrack {
            id: id.into(),
            title: title.into(),
            artist: artist.into(),
        },
        media,
    })
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    pub(crate) fn account_response() -> Value {
        json!({"header":{"activeAccountHeaderRenderer":{"accountName":{"runs":[{"text":"Fixture account"}]}}}})
    }
    fn player() -> Value {
        let expiry = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600;
        json!({"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"abcdefghijk","title":"Fixture","author":"Test"},"streamingData":{"adaptiveFormats":[{"mimeType":"audio/webm; codecs=opus","bitrate":128000,"url":format!("https://r1.googlevideo.com/videoplayback?expire={expiry}")}]}})
    }
    #[test]
    fn account_menu_must_contain_authenticated_account_details() {
        assert!(has_account_header(&account_response(), 0));
        assert!(!has_account_header(&json!({"actions":[]}), 0));
    }
    #[test]
    fn stream_requires_playability_correct_video_and_a_direct_supported_audio_lease() {
        assert!(parse_stream(player(), "abcdefghijk").is_ok());
        let mut denied = player();
        denied["playabilityStatus"]["status"] = "LOGIN_REQUIRED".into();
        assert!(parse_stream(denied, "abcdefghijk").is_err());
        assert!(parse_stream(player(), "differentid").is_err());
        let mut cipher = player();
        cipher["streamingData"]["adaptiveFormats"][0]["signatureCipher"] = "unsupported".into();
        assert!(parse_stream(cipher, "abcdefghijk").is_err());
    }
    #[test]
    fn ads_and_protected_streams_are_not_silently_skipped() {
        for key in ["adPlacements", "playerAds", "adSlots"] {
            let mut ads = player();
            ads[key] = json!([{"required":true}]);
            assert!(parse_stream(ads, "abcdefghijk")
                .err()
                .expect("Expected native playback rejection")
                .contains("ads"));
        }
        let mut drm = player();
        drm["streamingData"]["licenseInfos"] = json!([{}]);
        assert!(parse_stream(drm, "abcdefghijk").is_err());
    }
    #[test]
    fn bounded_search_only_collects_playable_track_identifiers_without_duplicates() {
        let item = json!({"musicTwoRowItemRenderer":{"title":{"runs":[{"text":"Track"}]},"navigationEndpoint":{"watchEndpoint":{"videoId":"abcdefghijk"}}}});
        let mut items = vec![item.clone(); 30];
        items.push(json!({"musicTwoRowItemRenderer":{"title":{"simpleText":"Album"},"navigationEndpoint":{"browseEndpoint":{"browseId":"not-a-track"}}}}));
        let mut tracks = Vec::new();
        collect_tracks(&json!({"items":items}), 0, &mut HashSet::new(), &mut tracks);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].title, "Track");
    }
}
