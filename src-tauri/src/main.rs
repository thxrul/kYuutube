#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use kyuutube_audio_spike::{Player, Snapshot};
use kyuutube_youtube_account::{Account, AccountStatus, Channel, ClientConfig, Page};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::State;

struct Audio(Mutex<Option<Player>>);

fn with_player<T>(
    audio: &State<Audio>,
    f: impl FnOnce(&Player) -> Result<T, String>,
) -> Result<T, String> {
    let guard = audio.0.lock().map_err(|_| "Audio state unavailable")?;
    f(guard.as_ref().ok_or("Start the local sample first")?)
}

#[tauri::command]
fn play_sample(audio: State<Audio>) -> Result<(), String> {
    let sample = PathBuf::from(
        std::env::var_os("KYUUTUBE_SPIKE_SAMPLE").ok_or("No local sample configured")?,
    );
    let mut guard = audio.0.lock().map_err(|_| "Audio state unavailable")?;
    if guard.is_none() {
        *guard = Some(Player::new(
            std::env::var_os("KYUUTUBE_NULL_OUTPUT").is_some(),
        )?);
    }
    guard
        .as_ref()
        .ok_or("Audio engine unavailable")?
        .load(&sample)
}

#[tauri::command]
fn pause(audio: State<Audio>, paused: bool) -> Result<(), String> {
    with_player(&audio, |p| p.pause(paused))
}
#[tauri::command]
fn seek(audio: State<Audio>, seconds: f64) -> Result<(), String> {
    with_player(&audio, |p| p.seek(seconds))
}
#[tauri::command]
fn volume(audio: State<Audio>, value: f64) -> Result<(), String> {
    with_player(&audio, |p| p.volume(value))
}
#[tauri::command]
fn snapshot(audio: State<Audio>) -> Result<Snapshot, String> {
    with_player(&audio, |p| Ok(p.snapshot()))
}

// Blocking network, native keyring, and browser callback work never runs on the UI thread.
#[tauri::command]
async fn account_status(account: State<'_, Arc<Account>>) -> Result<AccountStatus, String> {
    let account = account.inner().clone();
    tauri::async_runtime::spawn_blocking(move || account.status())
        .await
        .map_err(|_| "Account task failed")?
}
#[tauri::command]
async fn configure_google(
    account: State<'_, Arc<Account>>,
    client_id: String,
    client_secret: String,
) -> Result<(), String> {
    let account = account.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        account.configure(ClientConfig {
            client_id,
            client_secret,
        })
    })
    .await
    .map_err(|_| "Account task failed")?
}
#[tauri::command]
async fn google_sign_in(account: State<'_, Arc<Account>>) -> Result<Channel, String> {
    let account = account.inner().clone();
    tauri::async_runtime::spawn_blocking(move || account.sign_in())
        .await
        .map_err(|_| "Account task failed")?
}
#[tauri::command]
fn cancel_google_sign_in(account: State<Arc<Account>>) {
    account.cancel();
}
#[tauri::command]
async fn youtube_channel(account: State<'_, Arc<Account>>) -> Result<Channel, String> {
    let account = account.inner().clone();
    tauri::async_runtime::spawn_blocking(move || account.channel())
        .await
        .map_err(|_| "Account task failed")?
}
#[tauri::command]
async fn youtube_playlists(
    account: State<'_, Arc<Account>>,
    next_page: Option<String>,
) -> Result<Page, String> {
    let account = account.inner().clone();
    tauri::async_runtime::spawn_blocking(move || account.playlists(next_page))
        .await
        .map_err(|_| "Account task failed")?
}
#[tauri::command]
async fn disconnect_google(account: State<'_, Arc<Account>>) -> Result<(), String> {
    let account = account.inner().clone();
    tauri::async_runtime::spawn_blocking(move || account.disconnect())
        .await
        .map_err(|_| "Account task failed")?
}

fn main() {
    let account = Arc::new(Account::new().expect("Secure Google client could not initialize"));
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(target_os = "linux")]
            if std::env::var_os("KYUUTUBE_SPIKE_SOFTWARE_RENDER").is_some() {
                use tauri::Manager;
                use webkit2gtk::{SettingsExt, WebViewExt};
                let window = app
                    .get_webview_window("main")
                    .ok_or_else(|| std::io::Error::other("Spike window unavailable"))?;
                window.with_webview(|view| {
                    if let Some(settings) = view.inner().settings() {
                        settings.set_hardware_acceleration_policy(
                            webkit2gtk::HardwareAccelerationPolicy::Never,
                        );
                        eprintln!(
                            "WebKit graphics policy: {:?}",
                            settings.hardware_acceleration_policy()
                        );
                    }
                })?;
            }
            #[cfg(not(target_os = "linux"))]
            let _ = app;
            Ok(())
        })
        .manage(Audio(Mutex::new(None)))
        .manage(account)
        .invoke_handler(tauri::generate_handler![
            play_sample,
            pause,
            seek,
            volume,
            snapshot,
            account_status,
            configure_google,
            google_sign_in,
            cancel_google_sign_in,
            youtube_channel,
            youtube_playlists,
            disconnect_google
        ])
        .run(tauri::generate_context!())
        .expect("Desktop spike could not start");
}
