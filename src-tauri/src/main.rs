#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod session_login;
use kyuutube_audio_spike::{Player, Snapshot};
use kyuutube_youtube_account::{Account, AccountStatus, Channel, MusicTrack, Page};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::State;

struct Audio {
    player: Mutex<Option<Player>>,
    request: Mutex<()>,
    cancelled: AtomicBool,
}

fn with_player<T>(
    audio: &State<Arc<Audio>>,
    f: impl FnOnce(&Player) -> Result<T, String>,
) -> Result<T, String> {
    let guard = audio
        .player
        .try_lock()
        .map_err(|_| "Audio state unavailable")?;
    f(guard.as_ref().ok_or("Start a track first")?)
}

#[tauri::command]
fn play_sample(audio: State<Arc<Audio>>) -> Result<(), String> {
    let _request = audio
        .request
        .try_lock()
        .map_err(|_| "A track is already loading")?;
    let sample = PathBuf::from(
        std::env::var_os("KYUUTUBE_SPIKE_SAMPLE").ok_or("No local sample configured")?,
    );
    let mut guard = audio
        .player
        .try_lock()
        .map_err(|_| "Audio state unavailable")?;
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
fn pause(audio: State<Arc<Audio>>, paused: bool) -> Result<(), String> {
    with_player(&audio, |p| p.pause(paused))
}
#[tauri::command]
fn seek(audio: State<Arc<Audio>>, seconds: f64) -> Result<(), String> {
    with_player(&audio, |p| p.seek(seconds))
}
#[tauri::command]
fn volume(audio: State<Arc<Audio>>, value: f64) -> Result<(), String> {
    with_player(&audio, |p| p.volume(value))
}
#[tauri::command]
fn snapshot(audio: State<Arc<Audio>>) -> Result<Snapshot, String> {
    with_player(&audio, |p| Ok(p.snapshot()))
}

#[tauri::command]
async fn search_music(
    account: State<'_, Arc<Account>>,
    query: String,
) -> Result<Vec<MusicTrack>, String> {
    let account = account.inner().clone();
    tauri::async_runtime::spawn_blocking(move || account.search_music(query))
        .await
        .map_err(|_| "Music task failed")?
}
#[tauri::command]
async fn play_music(
    account: State<'_, Arc<Account>>,
    audio: State<'_, Arc<Audio>>,
    id: String,
) -> Result<MusicTrack, String> {
    let account = account.inner().clone();
    let audio = audio.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _request = audio
            .request
            .try_lock()
            .map_err(|_| "A track is already loading")?;
        audio.cancelled.store(false, Ordering::SeqCst);
        {
            let player = audio.player.lock().map_err(|_| "Audio state unavailable")?;
            if let Some(player) = player.as_ref() {
                player.stop()?;
            }
        }
        let stream = account.resolve_music_stream(id)?;
        if audio.cancelled.load(Ordering::SeqCst) {
            return Err("Playback cancelled".into());
        }
        {
            let mut player = audio.player.lock().map_err(|_| "Audio state unavailable")?;
            if player.is_none() {
                *player = Some(Player::new(false)?);
            }
            player
                .as_ref()
                .ok_or("Audio state unavailable")?
                .load_stream(&stream.media)?;
        }
        // IPC success means the decoder actually advanced, not just that an
        // asynchronous loadfile command was accepted. Do not hold the engine
        // mutex while waiting; pause/stop and the UI must remain responsive.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while std::time::Instant::now() < deadline {
            if audio.cancelled.load(Ordering::SeqCst) {
                if let Ok(player) = audio.player.lock() {
                    if let Some(player) = player.as_ref() {
                        let _ = player.stop();
                    }
                }
                return Err("Playback cancelled".into());
            }
            let player = audio.player.lock().map_err(|_| "Audio state unavailable")?;
            if player.as_ref().is_some_and(|player| {
                player
                    .snapshot()
                    .position_seconds
                    .is_some_and(|position| position > 0.05)
            }) {
                return Ok(stream.track);
            }
            drop(player);
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        if let Ok(player) = audio.player.lock() {
            if let Some(player) = player.as_ref() {
                let _ = player.stop();
            }
        }
        Err("The native player could not start this track".into())
    })
    .await
    .map_err(|_| "Playback task failed")?
}
#[tauri::command]
fn stop_music(audio: State<Arc<Audio>>) -> Result<(), String> {
    audio.cancelled.store(true, Ordering::SeqCst);
    let player = audio
        .player
        .try_lock()
        .map_err(|_| "Audio is busy; try stopping again")?;
    if let Some(player) = player.as_ref() {
        player.stop()?;
    }
    Ok(())
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
async fn google_sign_in(
    window: tauri::WebviewWindow,
    account: State<'_, Arc<Account>>,
) -> Result<(), String> {
    let account = account.inner().clone();
    let result = tauri::async_runtime::spawn_blocking(move || account.sign_in())
        .await
        .map_err(|_| "Account task failed")?;
    if result.is_ok() {
        // Bring the app back after consent. Focus requests are best-effort on
        // desktops that prevent applications from taking focus.
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    result
}
#[tauri::command]
async fn restore_google_session(account: State<'_, Arc<Account>>) -> Result<(), String> {
    let account = account.inner().clone();
    tauri::async_runtime::spawn_blocking(move || account.restore_session())
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

include!(concat!(env!("OUT_DIR"), "/publisher_oauth.rs"));

fn main() {
    let account = Arc::new(
        Account::new(publisher_client()).expect("Secure Google client could not initialize"),
    );
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
        .manage(Arc::new(Audio {
            player: Mutex::new(None),
            request: Mutex::new(()),
            cancelled: AtomicBool::new(false),
        }))
        .manage(account)
        .invoke_handler(|invoke| {
            if invoke.message.webview().label() != "main" {
                invoke
                    .resolver
                    .reject("Remote windows cannot invoke application commands");
                return true;
            }
            let handler: fn(tauri::ipc::Invoke<tauri::Wry>) -> bool = tauri::generate_handler![
                session_login::begin_music_login,
                session_login::finish_music_login,
                session_login::cancel_music_login,
                play_sample,
                pause,
                seek,
                volume,
                snapshot,
                search_music,
                play_music,
                stop_music,
                account_status,
                google_sign_in,
                restore_google_session,
                cancel_google_sign_in,
                youtube_channel,
                youtube_playlists,
                disconnect_google
            ];
            handler(invoke)
        })
        .run(tauri::generate_context!())
        .expect("Desktop spike could not start");
}
