//! Temporary account-login window only. Cookies remain in Rust and are committed
//! to the OS keystore only after Music account verification. No site script injection.
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
pub async fn begin_music_login(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("music-login") {
        return window
            .set_focus()
            .map_err(|_| "Could not open login".into());
    }
    WebviewWindowBuilder::new(&app,"music-login",WebviewUrl::External(Url::parse("https://accounts.google.com/ServiceLogin?continue=https%3A%2F%2Fmusic.youtube.com%2F").map_err(|_|"Invalid login address")?))
        .title("Sign in to YouTube Music").inner_size(700.0,800.0).incognito(true).on_navigation(allowed).build().map_err(|_|"Could not open login")?;
    Ok(())
}
#[tauri::command]
pub async fn finish_music_login(
    app: tauri::AppHandle,
    account: tauri::State<'_, std::sync::Arc<kyuutube_youtube_account::Account>>,
) -> Result<(), String> {
    let window = app
        .get_webview_window("music-login")
        .ok_or("Open the sign-in window first")?;
    let account = account.inner().clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let cookies = window
            .cookies_for_url(Url::parse(MUSIC).map_err(|_| "Invalid login address")?)
            .map_err(|_| "Could not read this app's login session")?;
        let header = cookies
            .iter()
            .map(|cookie| format!("{}={}", cookie.name(), cookie.value()))
            .collect::<Vec<_>>()
            .join("; ");
        account.save_music_session(header)?;
        window
            .close()
            .map_err(|_| "Account saved; close the login window".to_string())
    })
    .await
    .map_err(|_| "Login task failed")?
}
#[tauri::command]
pub async fn cancel_music_login(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("music-login") {
        window.close().map_err(|_| "Could not close login")?;
    }
    Ok(())
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
