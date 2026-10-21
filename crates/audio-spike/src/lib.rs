//! Native audio engine. Network input is restricted to expiring Google media leases.
use libloading::Library;
use serde::Serialize;
use std::{
    ffi::{c_char, c_int, c_void, CStr, CString},
    path::{Path, PathBuf},
    ptr::NonNull,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

type Create = unsafe extern "C" fn() -> *mut c_void;
type Initialize = unsafe extern "C" fn(*mut c_void) -> c_int;
type OptionString = unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char) -> c_int;
type Command = unsafe extern "C" fn(*mut c_void, *const *const c_char) -> c_int;
type GetProperty = unsafe extern "C" fn(*mut c_void, *const c_char, c_int, *mut c_void) -> c_int;
type Destroy = unsafe extern "C" fn(*mut c_void);
type ErrorString = unsafe extern "C" fn(c_int) -> *const c_char;

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub position_seconds: Option<f64>,
    pub duration_seconds: Option<f64>,
    pub paused: bool,
    pub audio_output: String,
}

// This type is deliberately not serializable or Debug: signed media URLs must
// never be returned to JavaScript, saved to disk, or included in logs.
pub struct RemoteMedia {
    source: zeroize::Zeroizing<String>,
    expires_at: SystemTime,
}
impl RemoteMedia {
    pub fn google_audio(source: String) -> Result<Self, String> {
        if source.len() > 16384 {
            return Err("Invalid media address".into());
        }
        let parsed = url::Url::parse(&source).map_err(|_| "Invalid media address")?;
        let host = parsed.host_str().ok_or("Invalid media host")?;
        if parsed.scheme() != "https"
            || !host.ends_with(".googlevideo.com")
            || parsed.port_or_known_default() != Some(443)
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.fragment().is_some()
            || parsed.path() != "/videoplayback"
        {
            return Err("Unsupported media address".into());
        }
        let expiry: Vec<_> = parsed
            .query_pairs()
            .filter(|(key, _)| key == "expire")
            .collect();
        if expiry.len() != 1 {
            return Err("Media expiry is unavailable".into());
        }
        let seconds: u64 = expiry[0].1.parse().map_err(|_| "Invalid media expiry")?;
        let expires_at = UNIX_EPOCH
            .checked_add(Duration::from_secs(seconds))
            .ok_or("Invalid media expiry")?;
        let remaining = expires_at
            .duration_since(SystemTime::now())
            .map_err(|_| "Media address expired")?;
        if remaining < Duration::from_secs(30) || remaining > Duration::from_secs(6 * 3600 + 60) {
            return Err("Media address expired or has an invalid lifetime".into());
        }
        Ok(Self {
            source: zeroize::Zeroizing::new(source),
            expires_at,
        })
    }
    fn usable(&self) -> bool {
        self.expires_at
            .duration_since(SystemTime::now())
            .is_ok_and(|time| time > Duration::from_secs(5))
    }
}

pub struct Player {
    handle: NonNull<c_void>,
    option: OptionString,
    command: Command,
    get: GetProperty,
    destroy: Destroy,
    error_string: ErrorString,
    audio_output: String,
    // Function pointers and the handle are destroyed before the library unloads.
    _library: Library,
}

// libmpv's client API is thread-safe. The desktop also serializes every access
// through a Mutex. This wrapper exposes neither callbacks nor the render API.
unsafe impl Send for Player {}

pub fn local_media(path: &Path) -> Result<PathBuf, String> {
    let resolved = path
        .canonicalize()
        .map_err(|_| "Local sample file is unavailable")?;
    if !resolved.is_file() {
        return Err("Sample must be a regular local file".into());
    }
    let extension = resolved
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !["wav", "flac", "mp3", "ogg", "m4a", "opus"].contains(&extension.as_str()) {
        return Err("Unsupported local sample format".into());
    }
    Ok(resolved)
}

fn library_paths() -> Vec<PathBuf> {
    if let Some(path) = std::env::var_os("KYUUTUBE_MPV_LIBRARY") {
        let path = PathBuf::from(path);
        // Explicit override only; never search the current working directory.
        return if path.is_absolute() {
            vec![path]
        } else {
            vec![]
        };
    }
    let mut paths = vec![];
    if let Ok(executable) = std::env::current_exe() {
        if let Some(parent) = executable.parent() {
            for name in ["libmpv.so.2", "libmpv.2.dylib", "libmpv-2.dll"] {
                paths.push(parent.join(name));
            }
        }
    }
    #[cfg(target_os = "linux")]
    paths.extend(
        [
            "/usr/lib/x86_64-linux-gnu/libmpv.so.2",
            "/usr/lib/aarch64-linux-gnu/libmpv.so.2",
            "/usr/lib/libmpv.so.2",
        ]
        .map(PathBuf::from),
    );
    #[cfg(target_os = "macos")]
    paths.extend(
        [
            "/opt/homebrew/lib/libmpv.2.dylib",
            "/usr/local/lib/libmpv.2.dylib",
        ]
        .map(PathBuf::from),
    );
    paths
}

impl Player {
    pub fn new(null_output: bool) -> Result<Self, String> {
        let library = library_paths()
            .into_iter()
            .filter(|p| p.is_file())
            .find_map(|p| {
                // Load only a bundled/system path or an explicit absolute override.
                unsafe { Library::new(p).ok() }
            })
            .ok_or("libmpv is unavailable; install the platform runtime for this spike")?;
        unsafe {
            let create = *library
                .get::<Create>(b"mpv_create\0")
                .map_err(|_| "Missing mpv_create")?;
            let initialize = *library
                .get::<Initialize>(b"mpv_initialize\0")
                .map_err(|_| "Missing mpv_initialize")?;
            let option = *library
                .get::<OptionString>(b"mpv_set_option_string\0")
                .map_err(|_| "Missing mpv_set_option_string")?;
            let command = *library
                .get::<Command>(b"mpv_command\0")
                .map_err(|_| "Missing mpv_command")?;
            let get = *library
                .get::<GetProperty>(b"mpv_get_property\0")
                .map_err(|_| "Missing mpv_get_property")?;
            let destroy = *library
                .get::<Destroy>(b"mpv_terminate_destroy\0")
                .map_err(|_| "Missing mpv_terminate_destroy")?;
            let error_string = *library
                .get::<ErrorString>(b"mpv_error_string\0")
                .map_err(|_| "Missing mpv_error_string")?;
            let handle = NonNull::new(create()).ok_or("mpv_create failed")?;
            let player = Self {
                handle,
                option,
                command,
                get,
                destroy,
                error_string,
                audio_output: if null_output {
                    "null (decoder only)"
                } else {
                    "system default"
                }
                .into(),
                _library: library,
            };
            for (key, value) in [
                ("config", "no"),
                ("terminal", "no"),
                ("video", "no"),
                ("audio-display", "no"),
                ("ytdl", "no"),
                ("load-scripts", "no"),
                ("tls-verify", "yes"),
                ("cache-secs", "20"),
                ("demuxer-max-bytes", "8MiB"),
                ("demuxer-max-back-bytes", "2MiB"),
                ("keep-open", "yes"),
                ("idle", "yes"),
            ] {
                player.set_option(key, value)?;
            }
            if null_output {
                player.set_option("ao", "null")?;
            }
            player.check(initialize(handle.as_ptr()))?;
            Ok(player)
        }
    }

    fn check(&self, status: c_int) -> Result<(), String> {
        if status >= 0 {
            return Ok(());
        }
        let message = unsafe { CStr::from_ptr((self.error_string)(status)) }.to_string_lossy();
        Err(format!("libmpv: {message}"))
    }

    fn set_option(&self, key: &str, value: &str) -> Result<(), String> {
        let key = CString::new(key).map_err(|_| "Invalid option")?;
        let value = CString::new(value).map_err(|_| "Invalid option value")?;
        self.check(unsafe { (self.option)(self.handle.as_ptr(), key.as_ptr(), value.as_ptr()) })
    }

    pub fn command(&self, arguments: &[&str]) -> Result<(), String> {
        let strings: Vec<CString> = arguments
            .iter()
            .map(|s| CString::new(*s))
            .collect::<Result<_, _>>()
            .map_err(|_| "Invalid command")?;
        let mut pointers: Vec<*const c_char> = strings.iter().map(|s| s.as_ptr()).collect();
        pointers.push(std::ptr::null());
        self.check(unsafe { (self.command)(self.handle.as_ptr(), pointers.as_ptr()) })
    }

    pub fn load(&self, path: &Path) -> Result<(), String> {
        let path = local_media(path)?;
        self.command(&[
            "loadfile",
            path.to_str().ok_or("Sample path is not valid UTF-8")?,
            "replace",
        ])?;
        self.command(&["set", "pause", "no"])
    }

    pub fn load_stream(&self, media: &RemoteMedia) -> Result<(), String> {
        if !media.usable() {
            return Err("Media address expired; choose the track again".into());
        }
        self.command(&["loadfile", &media.source, "replace"])?;
        self.command(&["set", "pause", "no"])
    }
    pub fn stop(&self) -> Result<(), String> {
        self.command(&["stop"])
    }

    pub fn pause(&self, paused: bool) -> Result<(), String> {
        self.command(&["set", "pause", if paused { "yes" } else { "no" }])
    }

    pub fn seek(&self, seconds: f64) -> Result<(), String> {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err("Seek position must be finite and nonnegative".into());
        }
        self.command(&["seek", &seconds.to_string(), "absolute"])
    }

    pub fn volume(&self, value: f64) -> Result<(), String> {
        if !value.is_finite() || !(0.0..=100.0).contains(&value) {
            return Err("Volume must be between 0 and 100".into());
        }
        self.command(&["set", "volume", &value.to_string()])
    }

    fn number(&self, name: &str) -> Option<f64> {
        let name = CString::new(name).ok()?;
        let mut value = 0.0_f64;
        let status = unsafe {
            (self.get)(
                self.handle.as_ptr(),
                name.as_ptr(),
                5,
                (&mut value as *mut f64).cast(),
            )
        };
        (status >= 0 && value.is_finite()).then_some(value)
    }

    pub fn snapshot(&self) -> Snapshot {
        let mut paused = 0_i32;
        unsafe {
            (self.get)(
                self.handle.as_ptr(),
                c"pause".as_ptr(),
                3,
                (&mut paused as *mut i32).cast(),
            );
        }
        Snapshot {
            position_seconds: self.number("time-pos"),
            duration_seconds: self.number("duration"),
            paused: paused != 0,
            audio_output: self.audio_output.clone(),
        }
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        unsafe {
            (self.destroy)(self.handle.as_ptr());
        }
    }
}
