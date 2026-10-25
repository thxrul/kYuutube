//! Cookies from the app's own temporary login window only. They never cross IPC
//! into JavaScript or touch plaintext files. Windows credential limits require
//! bounded chunks; a final pointer commits a new generation atomically.
use super::Account;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroizing;
#[derive(Serialize, Deserialize)]
struct Generation {
    id: String,
    count: usize,
}
pub(super) fn authorization(cookie: &str) -> Result<Zeroizing<String>, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "Invalid clock")?
        .as_secs();
    sign(cookie, now)
}
fn sign(cookie: &str, timestamp: u64) -> Result<Zeroizing<String>, String> {
    if !cookie.is_ascii() || cookie.len() > 16384 || cookie.bytes().any(|b| b.is_ascii_control()) {
        return Err("Invalid session cookies".into());
    }
    let values: Vec<_> = cookie
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .filter(|(name, _)| *name == "SAPISID" || *name == "__Secure-3PAPISID")
        .collect();
    let value = values
        .iter()
        .find(|(name, _)| *name == "SAPISID")
        .or_else(|| values.first())
        .map(|(_, value)| *value)
        .filter(|v| !v.is_empty())
        .ok_or("Finish signing in to YouTube Music first")?;
    let input = Zeroizing::new(format!("{timestamp} {value} https://music.youtube.com"));
    let digest = Sha1::digest(input.as_bytes());
    Ok(Zeroizing::new(format!(
        "SAPISIDHASH {timestamp}_{digest:x}"
    )))
}
impl Account {
    fn session_generation(&self) -> Result<Option<Generation>, String> {
        self.store
            .read("music-session")?
            .map(|v| {
                serde_json::from_str::<Generation>(&v)
                    .map_err(|_| "Saved session is invalid".into())
            })
            .transpose()
    }
    pub(super) fn session_cookie(&self) -> Result<Option<Zeroizing<String>>, String> {
        let Some(meta) = self.session_generation()? else {
            return Ok(None);
        };
        if meta.count == 0
            || meta.count > 8
            || meta.id.len() != 43
            || !meta
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err("Saved session is invalid".into());
        }
        let mut value = Zeroizing::new(String::new());
        for i in 0..meta.count {
            let chunk = self
                .store
                .read(&format!("music-{}-{i}", meta.id))?
                .ok_or("Saved session is incomplete")?;
            if chunk.len() > 2048 {
                return Err("Saved session is invalid".into());
            }
            value.push_str(&chunk);
        }
        sign(&value, 0)?;
        Ok(Some(value))
    }
    pub fn save_music_session(&self, cookie: String) -> Result<(), String> {
        let cookie = Zeroizing::new(cookie);
        let _guard = self
            .operation
            .try_lock()
            .map_err(|_| "An account operation is already running")?;
        let authorization = authorization(&cookie)?;
        let result = self.http.music_session(
            "account/account_menu",
            &cookie,
            &authorization,
            &serde_json::json!({"context":super::music::context()}),
        )?;
        if !super::music::has_account_header(&result, 0) {
            return Err("YouTube Music did not verify this account".into());
        }
        let previous = self.session_generation()?;
        let id = super::random_secret()?.to_string();
        let count = cookie.len().div_ceil(2048);
        let stored = (|| {
            for (i, chunk) in cookie.as_bytes().chunks(2048).enumerate() {
                self.store.write(
                    &format!("music-{id}-{i}"),
                    std::str::from_utf8(chunk).map_err(|_| "Invalid session")?,
                )?;
            }
            self.store.write(
                "music-session",
                &serde_json::to_string(&Generation {
                    id: id.clone(),
                    count,
                })
                .map_err(|_| "Invalid session")?,
            )
        })();
        if stored.is_err() {
            for i in 0..count {
                let _ = self.store.delete(&format!("music-{id}-{i}"));
            }
            return stored;
        }
        if let Some(old) = previous {
            for i in 0..old.count.min(8) {
                let _ = self.store.delete(&format!("music-{}-{i}", old.id));
            }
        }
        Ok(())
    }
    pub(super) fn forget_session(&self) -> Result<(), String> {
        if let Some(old) = self.session_generation()? {
            for i in 0..old.count.min(8) {
                self.store.delete(&format!("music-{}-{i}", old.id))?;
            }
        }
        self.store.delete("music-session")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn session_signing_is_origin_bound_and_rejects_header_injection() {
        let value = sign("SAPISID=fixture", 123).unwrap();
        let digest = Sha1::digest(b"123 fixture https://music.youtube.com");
        assert_eq!(value.as_str(), format!("SAPISIDHASH 123_{digest:x}"));
        assert!(sign("SAPISID=foo\r\nInjected=x", 123).is_err());
        assert!(sign("unrelated=x", 123).is_err());
    }
}
