//! Official-site test mode: no extraction, cookie reads, OAuth client, or injected
//! scripts. The remote window receives no remote IPC capabilities.
use tauri::{Manager, Url, WebviewUrl, WebviewWindowBuilder};
const MUSIC: &str = "https://music.youtube.com/";
fn allowed(url: &Url) -> bool {
    url.scheme() == "https"
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
        && matches!(
            url.host_str(),
            Some(
                "music.youtube.com"
                    | "www.youtube.com"
                    | "youtube.com"
                    | "accounts.google.com"
                    | "consent.google.com"
                    | "consent.youtube.com"
            )
        )
}
#[tauri::command]
pub fn open_music(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("official-music") {
        let _ = window.show();
        let _ = window.unminimize();
        return window
            .set_focus()
            .map_err(|_| "Could not open the music window".into());
    }
    WebviewWindowBuilder::new(
        &app,
        "official-music",
        WebviewUrl::External(Url::parse(MUSIC).map_err(|_| "Invalid music address")?),
    )
    .title("YouTube Music · kYuutube")
    .inner_size(1100.0, 780.0)
    .min_inner_size(400.0, 500.0)
    // Keep account cookies/session material out of persistent app files on
    // platforms whose webview cookie storage is not OS-keystore encrypted.
    .incognito(true)
    .on_navigation(allowed)
    .build()
    .map_err(|_| "Could not open the music window. Try Open in browser.")?;
    Ok(())
}
#[tauri::command]
pub fn open_music_browser() -> Result<(), String> {
    webbrowser::open(MUSIC)
        .map(|_| ())
        .map_err(|_| "Could not open your browser".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn official_navigation_keeps_login_and_music_origins_only() {
        for address in [
            MUSIC,
            "https://accounts.google.com/ServiceLogin",
            "https://consent.youtube.com/",
        ] {
            assert!(allowed(&Url::parse(address).unwrap()));
        }
        for address in [
            "http://music.youtube.com",
            "https://music.youtube.com.evil.invalid",
            "https://accounts.google.com@evil.invalid",
            "https://accounts.google.com:444/",
            "file:///etc/passwd",
            "tauri://localhost",
            "https://example.com",
        ] {
            assert!(!allowed(&Url::parse(address).unwrap()));
        }
    }
}
