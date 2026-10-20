#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use kyuutube_audio_spike::{Player, Snapshot};
use std::{path::PathBuf, sync::Mutex};
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

fn main() {
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
        .invoke_handler(tauri::generate_handler![
            play_sample,
            pause,
            seek,
            volume,
            snapshot
        ])
        .run(tauri::generate_context!())
        .expect("Desktop spike could not start");
}
