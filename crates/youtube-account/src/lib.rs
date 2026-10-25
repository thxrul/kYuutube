//! System-browser OAuth and an experimental authenticated Music service adapter.
//! Live service compatibility is unverified. Tokens and leases never enter the UI.
mod music;
mod session;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
pub use music::{MusicStream, MusicTrack};
use reqwest::blocking::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use url::Url;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

const SCOPE: &str = "https://www.googleapis.com/auth/youtube.readonly";
const MAX_BODY: u64 = 1024 * 1024;
const STORE_SERVICE: &str = "com.thxrul.kyuutube.google-desktop";

#[derive(Deserialize, Serialize, Zeroize, ZeroizeOnDrop)]
pub struct ClientConfig {
    pub client_id: String,
    pub client_secret: String,
}
impl ClientConfig {
    fn validate(&self) -> Result<(), String> {
        if !self.client_id.ends_with(".apps.googleusercontent.com")
            || self.client_id.len() > 256
            || self.client_id.chars().any(char::is_whitespace)
            || self.client_secret.len() > 256
            || self.client_secret.is_empty()
        {
            return Err(
                "Enter the client ID and secret for your Google Desktop app OAuth client".into(),
            );
        }
        Ok(())
    }
}

pub trait SecretStore: Send + Sync {
    fn read(&self, name: &str) -> Result<Option<Zeroizing<String>>, String>;
    fn write(&self, name: &str, value: &str) -> Result<(), String>;
    fn delete(&self, name: &str) -> Result<(), String>;
}
pub struct OsStore;
impl OsStore {
    fn entry(name: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(STORE_SERVICE, name)
            .map_err(|_| "OS credential store unavailable".into())
    }
}
impl SecretStore for OsStore {
    fn read(&self, name: &str) -> Result<Option<Zeroizing<String>>, String> {
        match Self::entry(name)?.get_password() {
            Ok(value) => Ok(Some(Zeroizing::new(value))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err("Unlock your OS credential store to connect your account".into()),
        }
    }
    fn write(&self, name: &str, value: &str) -> Result<(), String> {
        Self::entry(name)?.set_password(value).map_err(|_| {
            "Could not securely save credentials; no plaintext fallback is used".into()
        })
    }
    fn delete(&self, name: &str) -> Result<(), String> {
        match Self::entry(name)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err("Could not remove credentials from the OS store".into()),
        }
    }
}

#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    token_type: String,
    expires_in: u64,
    scope: Option<String>,
}
struct AccessToken {
    value: Zeroizing<String>,
    expires: Instant,
}
#[derive(Serialize)]
pub struct AccountStatus {
    pub available: bool,
    pub has_session: bool,
}
#[derive(Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub title: String,
}
#[derive(Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub subtitle: String,
}
#[derive(Serialize, Deserialize)]
pub struct Page {
    pub items: Vec<Item>,
    pub next_page: Option<String>,
}

// All production network destinations are fixed Google endpoints. Browser OAuth
// state/verifiers and HTTP response bodies are never included in error messages.
trait Transport: Send + Sync {
    fn form(&self, endpoint: &str, fields: &[(&str, &str)]) -> Result<Value, String>;
    fn get(&self, resource: &str, token: &str, query: &[(&str, &str)]) -> Result<Value, String>;
    fn music(&self, endpoint: &str, token: &str, body: &Value) -> Result<Value, String>;
    fn music_session(
        &self,
        _endpoint: &str,
        _cookie: &str,
        _authorization: &str,
        _body: &Value,
    ) -> Result<Value, String> {
        Err("Session transport unavailable".into())
    }
}
struct GoogleHttp(Client);
fn response_json(response: Response) -> Result<Value, String> {
    let status = response.status();
    let mut bytes = Zeroizing::new(Vec::new());
    response
        .take(MAX_BODY + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Google response could not be read")?;
    if bytes.len() as u64 > MAX_BODY {
        return Err("Google response exceeded the size limit".into());
    }
    if !status.is_success() {
        let error = serde_json::from_slice::<Value>(&bytes).unwrap_or_default();
        return Err(if error["error"] == "invalid_grant" {
            "Authorization expired or was revoked; sign in again"
        } else if status.as_u16() == 401 {
            "YouTube rejected authorization; sign in again"
        } else if status.as_u16() == 403 {
            "YouTube access denied; enable YouTube Data API v3 and check consent permissions and quota"
        } else { "Google request failed; try again later" }.into());
    }
    // Revocation returns an empty body on success.
    if bytes.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_slice(&bytes).map_err(|_| "Invalid Google response".into())
}
impl GoogleHttp {
    fn new() -> Result<Self, String> {
        Client::builder()
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map(Self)
            .map_err(|_| "Could not create a secure Google connection".into())
    }
}
impl Transport for GoogleHttp {
    fn form(&self, endpoint: &str, fields: &[(&str, &str)]) -> Result<Value, String> {
        let url = match endpoint {
            "token" => "https://oauth2.googleapis.com/token",
            "revoke" => "https://oauth2.googleapis.com/revoke",
            _ => return Err("Unsupported Google endpoint".into()),
        };
        response_json(
            self.0
                .post(url)
                .form(fields)
                .send()
                .map_err(|_| "Could not reach Google; check your connection")?,
        )
    }
    fn get(&self, resource: &str, token: &str, query: &[(&str, &str)]) -> Result<Value, String> {
        if !matches!(
            resource,
            "channels" | "playlists" | "playlistItems" | "search"
        ) {
            return Err("Unsupported YouTube endpoint".into());
        }
        response_json(
            self.0
                .get(format!("https://www.googleapis.com/youtube/v3/{resource}"))
                .bearer_auth(token)
                .query(query)
                .send()
                .map_err(|_| "Could not reach YouTube; check your connection")?,
        )
    }
    fn music(&self, endpoint: &str, token: &str, body: &Value) -> Result<Value, String> {
        if !matches!(endpoint, "account/account_menu" | "search" | "player") {
            return Err("Unsupported Music endpoint".into());
        }
        response_json(
            self.0
                .post(format!("https://music.youtube.com/youtubei/v1/{endpoint}"))
                .bearer_auth(token)
                .header("Origin", "https://music.youtube.com")
                .header("User-Agent", "kYuutube/0.0.1")
                .json(body)
                .send()
                .map_err(|_| "Could not reach YouTube Music; check your connection")?,
        )
    }
    fn music_session(
        &self,
        endpoint: &str,
        cookie: &str,
        authorization: &str,
        body: &Value,
    ) -> Result<Value, String> {
        if !matches!(endpoint, "account/account_menu" | "search" | "player") {
            return Err("Unsupported Music endpoint".into());
        }
        response_json(
            self.0
                .post(format!("https://music.youtube.com/youtubei/v1/{endpoint}"))
                .header("Cookie", cookie)
                .header("Authorization", authorization)
                .header("Origin", "https://music.youtube.com")
                .header("X-Goog-AuthUser", "0")
                .json(body)
                .send()
                .map_err(|_| "Could not reach YouTube Music; check your connection")?,
        )
    }
}

struct Attempt {
    listener: TcpListener,
    redirect: String,
    state: Zeroizing<String>,
    verifier: Zeroizing<String>,
    authorize: Url,
}
fn random_secret() -> Result<Zeroizing<String>, String> {
    let mut bytes = Zeroizing::new([0u8; 32]);
    getrandom::fill(bytes.as_mut()).map_err(|_| "Secure randomness unavailable")?;
    Ok(Zeroizing::new(URL_SAFE_NO_PAD.encode(bytes.as_ref())))
}
fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}
impl Attempt {
    fn new(config: &ClientConfig) -> Result<Self, String> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .map_err(|_| "Could not open the local sign-in callback")?;
        listener
            .set_nonblocking(true)
            .map_err(|_| "Local sign-in callback unavailable")?;
        let redirect = format!(
            "http://127.0.0.1:{}/oauth/callback",
            listener
                .local_addr()
                .map_err(|_| "Local callback unavailable")?
                .port()
        );
        let state = random_secret()?;
        let verifier = random_secret()?;
        let mut authorize = Url::parse("https://accounts.google.com/o/oauth2/v2/auth")
            .map_err(|_| "Invalid Google authorization endpoint")?;
        authorize.query_pairs_mut().extend_pairs([
            ("client_id", config.client_id.as_str()),
            ("redirect_uri", redirect.as_str()),
            ("response_type", "code"),
            ("scope", SCOPE),
            ("state", &state),
            ("code_challenge", &challenge(&verifier)),
            ("code_challenge_method", "S256"),
            ("access_type", "offline"),
            ("prompt", "consent"),
        ]);
        Ok(Self {
            listener,
            redirect,
            state,
            verifier,
            authorize,
        })
    }
    fn wait(&self, cancelled: &AtomicBool, timeout: Duration) -> Result<Zeroizing<String>, String> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline && !cancelled.load(Ordering::SeqCst) {
            match self.listener.accept() {
                Ok((mut stream, peer)) => {
                    if !peer.ip().is_loopback() {
                        continue;
                    }
                    stream
                        .set_read_timeout(Some(Duration::from_millis(500)))
                        .ok();
                    stream
                        .set_write_timeout(Some(Duration::from_millis(500)))
                        .ok();
                    let request = read_callback(&mut stream);
                    let result = request.and_then(|request| {
                        validate_callback(&request, &self.redirect, &self.state)
                    });
                    let text = if result.is_ok() {
                        "Finishing sign-in. You can return to kYuutube."
                    } else {
                        "Sign-in was not completed. Return to kYuutube to try again."
                    };
                    let status = if result.is_ok() {
                        "200 OK"
                    } else {
                        "400 Bad Request"
                    };
                    let _ = write!(stream, "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'; frame-ancestors 'none'\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{text}", text.len());
                    match result {
                        Ok(code) => return Ok(code),
                        Err(error) if error == "Google sign-in was declined" => return Err(error),
                        Err(_) => continue, // Stray callbacks cannot terminate the legitimate attempt.
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50))
                }
                Err(_) => return Err("Local sign-in listener failed".into()),
            }
        }
        Err(if cancelled.load(Ordering::SeqCst) {
            "Sign-in cancelled"
        } else {
            "Sign-in timed out; please try again"
        }
        .into())
    }
}
fn read_callback(stream: &mut TcpStream) -> Result<Zeroizing<String>, String> {
    let mut bytes = Zeroizing::new(Vec::new());
    let mut block = [0; 1024];
    let deadline = Instant::now() + Duration::from_secs(1);
    while bytes.len() < 8192 {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or("Callback read timed out")?;
        stream
            .set_read_timeout(Some(remaining))
            .map_err(|_| "Callback read unavailable")?;
        let count = stream
            .read(&mut block)
            .map_err(|_| "Invalid callback request")?;
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&block[..count]);
        if bytes.len() > 8192 {
            return Err("Callback request too large".into());
        }
        if bytes.windows(4).any(|part| part == b"\r\n\r\n") {
            return String::from_utf8(bytes.to_vec())
                .map(Zeroizing::new)
                .map_err(|_| "Invalid callback encoding".into());
        }
    }
    Err("Callback headers incomplete or too large".into())
}
fn validate_callback(
    request: &str,
    redirect: &str,
    state: &str,
) -> Result<Zeroizing<String>, String> {
    let invalid = || "Invalid sign-in callback".to_string();
    let mut lines = request.split("\r\n");
    let parts: Vec<_> = lines.next().unwrap_or_default().split(' ').collect();
    if parts.len() != 3
        || parts[0] != "GET"
        || parts[2] != "HTTP/1.1"
        || !parts[1].starts_with('/')
        || parts[1].starts_with("//")
    {
        return Err(invalid());
    }
    let base = Url::parse(redirect).map_err(|_| invalid())?;
    let callback = base.join(parts[1]).map_err(|_| invalid())?;
    let hosts: Vec<_> = lines
        .filter_map(|line| line.split_once(':'))
        .filter(|(name, _)| name.eq_ignore_ascii_case("host"))
        .collect();
    let expected_host = format!("127.0.0.1:{}", base.port().ok_or_else(invalid)?);
    if hosts.len() != 1
        || hosts[0].1.trim() != expected_host
        || callback.path() != base.path()
        || callback.origin() != base.origin()
        || callback.fragment().is_some()
    {
        return Err(invalid());
    }
    let fields: Vec<_> = callback.query_pairs().collect();
    let states: Vec<_> = fields.iter().filter(|(key, _)| key == "state").collect();
    if states.len() != 1 || !constant_time_equal(&states[0].1, state) {
        return Err(invalid());
    }
    if fields.iter().any(|(key, _)| key == "error") {
        return Err("Google sign-in was declined".into());
    }
    let codes: Vec<_> = fields.iter().filter(|(key, _)| key == "code").collect();
    if codes.len() != 1 || codes[0].1.is_empty() || codes[0].1.len() > 4096 {
        return Err(invalid());
    }
    Ok(Zeroizing::new(codes[0].1.to_string()))
}
fn constant_time_equal(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

pub struct Account {
    app_client: Option<ClientConfig>,
    store: Box<dyn SecretStore>,
    http: Box<dyn Transport>,
    operation: Mutex<()>,
    access: Mutex<Option<AccessToken>>,
    cancelled: AtomicBool,
}
impl Account {
    pub fn new(app_client: Option<ClientConfig>) -> Result<Self, String> {
        if let Some(config) = &app_client {
            config.validate()?;
        }
        Ok(Self::with(
            app_client,
            Box::new(OsStore),
            Box::new(GoogleHttp::new()?),
        ))
    }
    fn with(
        app_client: Option<ClientConfig>,
        store: Box<dyn SecretStore>,
        http: Box<dyn Transport>,
    ) -> Self {
        Self {
            app_client,
            store,
            http,
            operation: Mutex::new(()),
            access: Mutex::new(None),
            cancelled: AtomicBool::new(false),
        }
    }
    pub fn status(&self) -> Result<AccountStatus, String> {
        if self.session_cookie()?.is_some() {
            return Ok(AccountStatus {
                available: true,
                has_session: true,
            });
        }
        // A publisher registration is supplied by the build, never by the user.
        // Old alpha credentials cannot silently enable an unprovisioned build.
        let Some(config) = &self.app_client else {
            return Ok(AccountStatus {
                available: false,
                has_session: false,
            });
        };
        let matches = self.saved_client_matches(config)?;
        Ok(AccountStatus {
            available: true,
            has_session: matches && self.store.read("refresh")?.is_some(),
        })
    }
    fn saved_client_matches(&self, config: &ClientConfig) -> Result<bool, String> {
        let Some(value) = self.store.read("client")? else {
            return Ok(false);
        };
        Ok(serde_json::from_str::<ClientConfig>(&value)
            .is_ok_and(|saved| saved.client_id == config.client_id))
    }
    fn publisher_client(&self) -> Result<&ClientConfig, String> {
        let config = self
            .app_client
            .as_ref()
            .ok_or("Sign-in is not available in this build")?;
        config.validate()?;
        Ok(config)
    }
    fn prepare_client(&self) -> Result<(), String> {
        let config = self.publisher_client()?;
        // A cancelled retry must preserve the current grant. A changed publisher
        // registration must never reuse another client's refresh credential.
        if !self.saved_client_matches(config)? {
            self.store.delete("refresh")?;
            *self
                .access
                .lock()
                .map_err(|_| "Account state unavailable")? = None;
        }
        let value = Zeroizing::new(
            serde_json::to_string(config).map_err(|_| "Invalid client configuration")?,
        );
        self.store.write("client", &value)?;
        if self.store.read("client")?.as_deref().map(|v| v.as_str()) != Some(value.as_str()) {
            return Err("OS credential store could not verify the saved client".into());
        }
        Ok(())
    }
    fn config(&self) -> Result<&ClientConfig, String> {
        let config = self.publisher_client()?;
        if !self.saved_client_matches(config)? {
            return Err("Sign in with Google first".into());
        }
        Ok(config)
    }
    pub fn sign_in(&self) -> Result<(), String> {
        let _operation = self
            .operation
            .try_lock()
            .map_err(|_| "An account operation is already running")?;
        self.cancelled.store(false, Ordering::SeqCst);
        self.prepare_client()?;
        let config = self.config()?;
        let attempt = Attempt::new(config)?;
        webbrowser::open(attempt.authorize.as_str())
            .map_err(|_| "Could not open the system browser")?;
        let code = attempt.wait(&self.cancelled, Duration::from_secs(300))?;
        let response = self.http.form(
            "token",
            &[
                ("client_id", &config.client_id),
                ("client_secret", &config.client_secret),
                ("grant_type", "authorization_code"),
                ("code", &code),
                ("redirect_uri", &attempt.redirect),
                ("code_verifier", &attempt.verifier),
            ],
        )?;
        if self.cancelled.load(Ordering::SeqCst) {
            return Err("Sign-in cancelled".into());
        }
        self.finish_sign_in(response)
    }
    pub fn restore_session(&self) -> Result<(), String> {
        let _operation = self
            .operation
            .try_lock()
            .map_err(|_| "An account operation is already running")?;
        // Saved credentials are not proof of a valid session. On restart, verify
        // the refresh grant with Google before showing the user as signed in.
        self.verify_music_account().map(|_| ())
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
    fn decode_token(value: Value, require_refresh: bool) -> Result<TokenResponse, String> {
        let response: TokenResponse =
            serde_json::from_value(value).map_err(|_| "Invalid Google token response")?;
        if !response.token_type.eq_ignore_ascii_case("bearer")
            || response.access_token.is_empty()
            || response.access_token.len() > 16384
            || response.expires_in <= 60
            || response.expires_in > 86400
            || response
                .scope
                .as_ref()
                .is_some_and(|scope| !scope.split_whitespace().any(|scope| scope == SCOPE))
        {
            return Err("Google did not grant the required YouTube read permission".into());
        }
        let refresh = response
            .refresh_token
            .as_ref()
            .filter(|value| !value.is_empty());
        if refresh.is_some_and(|value| value.len() > 2048) {
            return Err("Google refresh credential exceeded secure storage limit".into());
        }
        if require_refresh && refresh.is_none() {
            return Err("Google did not provide offline authorization; sign in again".into());
        }
        Ok(response)
    }
    fn save_token(&self, response: &TokenResponse) -> Result<(), String> {
        if let Some(refresh) = response
            .refresh_token
            .as_ref()
            .filter(|value| !value.is_empty())
        {
            self.store.write("refresh", refresh)?;
        }
        *self
            .access
            .lock()
            .map_err(|_| "Account state unavailable")? = Some(AccessToken {
            value: Zeroizing::new(response.access_token.clone()),
            expires: Instant::now() + Duration::from_secs(response.expires_in - 60),
        });
        Ok(())
    }
    fn accept_token(&self, value: Value, require_refresh: bool) -> Result<(), String> {
        self.save_token(&Self::decode_token(value, require_refresh)?)
    }
    fn finish_sign_in(&self, value: Value) -> Result<(), String> {
        let response = Self::decode_token(value, true)?;
        if self.cancelled.load(Ordering::SeqCst) {
            return Err("Sign-in cancelled".into());
        }
        // Verify this new grant explicitly. Never let an older saved refresh
        // credential turn a failed new Music login into apparent success.
        let verified = self.verify_music_with_token(&response.access_token);
        if self.cancelled.load(Ordering::SeqCst) {
            return Err("Sign-in cancelled".into());
        }
        verified?;
        self.save_token(&response)
    }
    fn token(&self) -> Result<Zeroizing<String>, String> {
        if let Some(token) = self
            .access
            .lock()
            .map_err(|_| "Account state unavailable")?
            .as_ref()
            .filter(|token| token.expires > Instant::now())
        {
            return Ok(token.value.clone());
        }
        let config = self.config()?;
        let refresh = self
            .store
            .read("refresh")?
            .ok_or("Sign in with Google first")?;
        let result = self.http.form(
            "token",
            &[
                ("client_id", &config.client_id),
                ("client_secret", &config.client_secret),
                ("grant_type", "refresh_token"),
                ("refresh_token", &refresh),
            ],
        );
        match result {
            Ok(response) => self.accept_token(response, false)?,
            Err(error) => {
                if error.contains("revoked") {
                    self.store.delete("refresh")?;
                    *self
                        .access
                        .lock()
                        .map_err(|_| "Account state unavailable")? = None;
                }
                return Err(error);
            }
        }
        self.access
            .lock()
            .map_err(|_| "Account state unavailable")?
            .as_ref()
            .map(|token| token.value.clone())
            .ok_or("No access credential available".into())
    }
    fn get_authorized(&self, resource: &str, query: &[(&str, &str)]) -> Result<Value, String> {
        let result = self.http.get(resource, &self.token()?, query);
        if result
            .as_ref()
            .is_err_and(|error| error.starts_with("YouTube rejected authorization"))
        {
            *self
                .access
                .lock()
                .map_err(|_| "Account state unavailable")? = None;
            // A single refresh retry handles server-side expiry without a retry loop.
            self.http.get(resource, &self.token()?, query)
        } else {
            result
        }
    }
    fn channel_inner(&self) -> Result<Channel, String> {
        let value = self.get_authorized("channels", &[("part", "snippet"), ("mine", "true")])?;
        let item = value["items"]
            .as_array()
            .and_then(|items| items.first())
            .ok_or("Google connected, but no YouTube channel was returned")?;
        Ok(Channel {
            id: text(item, "id")?,
            title: text(&item["snippet"], "title")?,
        })
    }
    pub fn channel(&self) -> Result<Channel, String> {
        let _operation = self
            .operation
            .try_lock()
            .map_err(|_| "An account operation is already running")?;
        self.channel_inner()
    }
    pub fn playlists(&self, next: Option<String>) -> Result<Page, String> {
        let _operation = self
            .operation
            .try_lock()
            .map_err(|_| "An account operation is already running")?;
        let next = next.unwrap_or_default();
        if next.len() > 1024 {
            return Err("Invalid page cursor".into());
        }
        let value = self.get_authorized(
            "playlists",
            &[
                ("part", "snippet"),
                ("mine", "true"),
                ("maxResults", "25"),
                ("pageToken", &next),
            ],
        )?;
        parse_page(value)
    }
    pub fn disconnect(&self) -> Result<(), String> {
        self.cancel();
        let _operation = self
            .operation
            .try_lock()
            .map_err(|_| "Wait for the cancelled operation to finish, then disconnect")?;
        self.forget_session()?;
        let refresh = self.store.read("refresh")?;
        let remote = refresh
            .as_ref()
            .map(|token| self.http.form("revoke", &[("token", token)]))
            .transpose();
        self.store.delete("refresh")?;
        *self
            .access
            .lock()
            .map_err(|_| "Account state unavailable")? = None;
        self.store.delete("client")?;
        if remote.is_err() {
            return Err("Local credentials removed. Google revocation failed; remove kYuutube access in your Google Account permissions".into());
        }
        Ok(())
    }
}
fn text(value: &Value, key: &str) -> Result<String, String> {
    let text = value[key]
        .as_str()
        .ok_or("YouTube returned incomplete metadata")?;
    if text.len() > 8192 {
        return Err("YouTube metadata exceeded the size limit".into());
    }
    Ok(text.to_owned())
}
fn parse_page(value: Value) -> Result<Page, String> {
    let items = value["items"]
        .as_array()
        .ok_or("YouTube returned an invalid playlist page")?;
    if items.len() > 25 {
        return Err("YouTube playlist page exceeded the item limit".into());
    }
    let items = items
        .iter()
        .map(|item| {
            Ok(Item {
                id: text(item, "id")?,
                title: text(&item["snippet"], "title")?,
                subtitle: text(&item["snippet"], "channelTitle")?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let next_page = value["nextPageToken"].as_str().map(str::to_owned);
    if next_page.as_ref().is_some_and(|next| next.len() > 1024) {
        return Err("Invalid YouTube page cursor".into());
    }
    Ok(Page { items, next_page })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        collections::{HashMap, VecDeque},
        sync::Arc,
    };

    #[derive(Default)]
    struct MemoryStore(Mutex<HashMap<String, String>>);
    impl SecretStore for Arc<MemoryStore> {
        fn read(&self, name: &str) -> Result<Option<Zeroizing<String>>, String> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .get(name)
                .cloned()
                .map(Zeroizing::new))
        }
        fn write(&self, name: &str, value: &str) -> Result<(), String> {
            self.0.lock().unwrap().insert(name.into(), value.into());
            Ok(())
        }
        fn delete(&self, name: &str) -> Result<(), String> {
            self.0.lock().unwrap().remove(name);
            Ok(())
        }
    }
    #[derive(Default)]
    struct FakeHttp {
        forms: Mutex<VecDeque<Result<Value, String>>>,
        gets: Mutex<VecDeque<Result<Value, String>>>,
        calls: Mutex<Vec<String>>,
    }
    impl Transport for Arc<FakeHttp> {
        fn form(&self, endpoint: &str, _: &[(&str, &str)]) -> Result<Value, String> {
            self.calls.lock().unwrap().push(endpoint.into());
            self.forms
                .lock()
                .unwrap()
                .pop_front()
                .expect("Unexpected POST")
        }
        fn get(&self, resource: &str, _: &str, _: &[(&str, &str)]) -> Result<Value, String> {
            self.calls.lock().unwrap().push(resource.into());
            self.gets
                .lock()
                .unwrap()
                .pop_front()
                .expect("Unexpected GET")
        }
        fn music(&self, endpoint: &str, _: &str, _: &Value) -> Result<Value, String> {
            self.calls.lock().unwrap().push(format!("music:{endpoint}"));
            self.gets
                .lock()
                .unwrap()
                .pop_front()
                .expect("Unexpected Music POST")
        }
        fn music_session(
            &self,
            endpoint: &str,
            _: &str,
            _: &str,
            _: &Value,
        ) -> Result<Value, String> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("session:{endpoint}"));
            self.gets
                .lock()
                .unwrap()
                .pop_front()
                .expect("Unexpected session request")
        }
    }
    fn config() -> ClientConfig {
        ClientConfig {
            client_id: "test.apps.googleusercontent.com".into(),
            client_secret: "test-secret".into(),
        }
    }
    fn fixture() -> (Account, Arc<MemoryStore>, Arc<FakeHttp>) {
        let store = Arc::new(MemoryStore::default());
        let http = Arc::new(FakeHttp::default());
        let account = Account::with(
            Some(config()),
            Box::new(store.clone()),
            Box::new(http.clone()),
        );
        account.prepare_client().unwrap();
        (account, store, http)
    }
    fn token(refresh: bool) -> Value {
        let mut token = json!({"access_token":"fake-access", "token_type":"Bearer", "expires_in":3600, "scope": SCOPE});
        if refresh {
            token["refresh_token"] = "fake-refresh".into();
        }
        token
    }
    fn request(path: &str) -> String {
        format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:54321\r\n\r\n")
    }
    #[test]
    fn pkce_matches_rfc7636_and_attempt_is_bound_to_loopback() {
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        let a = Attempt::new(&config()).unwrap();
        let b = Attempt::new(&config()).unwrap();
        assert_ne!(*a.state, *b.state);
        assert_ne!(*a.verifier, *b.verifier);
        assert!(a.listener.local_addr().unwrap().ip().is_loopback());
        let fields: HashMap<_, _> = a.authorize.query_pairs().collect();
        assert_eq!(fields["code_challenge_method"], "S256");
        assert_eq!(fields["redirect_uri"], a.redirect);
        assert_eq!(fields["scope"], SCOPE);
    }
    #[test]
    fn callback_rejects_wrong_state_host_path_and_duplicate_parameters() {
        let redirect = "http://127.0.0.1:54321/oauth/callback";
        assert_eq!(
            *validate_callback(
                &request("/oauth/callback?state=correct&code=good"),
                redirect,
                "correct"
            )
            .unwrap(),
            "good"
        );
        for path in [
            "/oauth/callback?state=wrong&code=good",
            "/wrong?state=correct&code=good",
            "/oauth/callback?state=correct&state=correct&code=good",
            "/oauth/callback?state=correct&code=a&code=b",
            "//evil.invalid/oauth/callback?state=correct&code=good",
            "/oauth/callback?state=correct&code=",
        ] {
            assert!(
                validate_callback(&request(path), redirect, "correct").is_err(),
                "{path}"
            );
        }
        let hostile = request("/oauth/callback?state=correct&code=good")
            .replace("Host: 127.0.0.1:54321", "Host: evil.invalid");
        assert!(validate_callback(&hostile, redirect, "correct").is_err());
        assert_eq!(
            validate_callback(
                &request("/oauth/callback?state=correct&error=access_denied"),
                redirect,
                "correct"
            )
            .unwrap_err(),
            "Google sign-in was declined"
        );
    }
    #[test]
    fn real_loopback_receives_callback_and_cancel_and_timeout_are_bounded() {
        let attempt = Attempt::new(&config()).unwrap();
        let addr = attempt.listener.local_addr().unwrap();
        let state = attempt.state.to_string();
        let sender = thread::spawn(move || {
            let mut stream = TcpStream::connect(addr).unwrap();
            write!(stream, "GET /oauth/callback?state={state}&code=local-test HTTP/1.1\r\nHost: {addr}\r\n\r\n").unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            assert!(response.starts_with("HTTP/1.1 200"));
            assert!(!response.contains("local-test"));
        });
        assert_eq!(
            *attempt
                .wait(&AtomicBool::new(false), Duration::from_secs(2))
                .unwrap(),
            "local-test"
        );
        sender.join().unwrap();
        assert!(attempt
            .wait(&AtomicBool::new(true), Duration::from_secs(2))
            .unwrap_err()
            .contains("cancelled"));
        assert!(attempt
            .wait(&AtomicBool::new(false), Duration::from_millis(10))
            .unwrap_err()
            .contains("timed out"));
    }
    #[test]
    fn refresh_is_securely_persisted_but_access_token_is_only_in_memory() {
        let (account, store, http) = fixture();
        account.accept_token(token(true), true).unwrap();
        assert_eq!(&**store.read("refresh").unwrap().unwrap(), "fake-refresh");
        assert!(!store
            .0
            .lock()
            .unwrap()
            .values()
            .any(|value| value.contains("fake-access")));
        assert_eq!(&*account.token().unwrap(), "fake-access");
        assert!(http.calls.lock().unwrap().is_empty());
        account.access.lock().unwrap().as_mut().unwrap().expires = Instant::now();
        http.forms.lock().unwrap().push_back(Ok(token(false)));
        assert_eq!(&*account.token().unwrap(), "fake-access");
        assert_eq!(&*http.calls.lock().unwrap(), &["token"]);
        assert!(account.status().unwrap().has_session);
    }
    #[test]
    fn revoked_refresh_clears_session_but_transient_network_error_preserves_it() {
        let (account, store, http) = fixture();
        store.write("refresh", "fake-refresh").unwrap();
        http.forms
            .lock()
            .unwrap()
            .push_back(Err("Could not reach Google".into()));
        assert!(account.token().is_err());
        assert!(store.read("refresh").unwrap().is_some());
        http.forms.lock().unwrap().push_back(Err(
            "Authorization expired or was revoked; sign in again".into(),
        ));
        assert!(account.token().is_err());
        assert!(store.read("refresh").unwrap().is_none());
    }
    #[test]
    fn invalid_token_or_missing_scope_is_not_persisted() {
        let (account, store, _) = fixture();
        for invalid in [
            json!({"access_token":"a","token_type":"Bearer","expires_in":3600,"scope":"email","refresh_token":"bad"}),
            json!({"access_token":"a","token_type":"Bearer","expires_in":0,"refresh_token":"bad"}),
            token(false),
        ] {
            assert!(account.accept_token(invalid, true).is_err());
            assert!(store.read("refresh").unwrap().is_none());
        }
    }
    #[test]
    fn changing_client_clears_old_grant_and_credentials_are_not_returned_to_ui() {
        let (account, store, _) = fixture();
        account.accept_token(token(true), true).unwrap();
        let changed = ClientConfig {
            client_id: "another.apps.googleusercontent.com".into(),
            client_secret: "another-secret".into(),
        };
        let account = Account::with(
            Some(changed),
            Box::new(store.clone()),
            Box::new(Arc::new(FakeHttp::default())),
        );
        assert!(!account.status().unwrap().has_session);
        account.prepare_client().unwrap();
        assert!(store.read("refresh").unwrap().is_none());
        assert!(account.access.lock().unwrap().is_none());
        let public = serde_json::to_string(&account.status().unwrap()).unwrap();
        assert!(!public.contains("test-secret") && !public.contains("fake-refresh"));
    }
    #[test]
    fn disconnect_removes_local_credentials_even_when_google_is_offline() {
        let (account, store, http) = fixture();
        account.accept_token(token(true), true).unwrap();
        http.forms.lock().unwrap().push_back(Err("offline".into()));
        assert!(account
            .disconnect()
            .unwrap_err()
            .contains("Local credentials removed"));
        assert!(store.0.lock().unwrap().is_empty());
        assert!(account.status().unwrap().available);
        assert!(!account.status().unwrap().has_session);
        assert!(account.access.lock().unwrap().is_none());
    }
    #[test]
    fn empty_and_paginated_playlists_are_bounded() {
        assert!(parse_page(json!({"items":[]})).unwrap().items.is_empty());
        let page = parse_page(json!({"items":[{"id":"one","snippet":{"title":"Playlist","channelTitle":"Owner"}}],"nextPageToken":"cursor"})).unwrap();
        assert_eq!(page.next_page.as_deref(), Some("cursor"));
        assert_eq!(page.items[0].id, "one");
        assert!(parse_page(json!({"items":vec![json!({});26]})).is_err());
        assert!(parse_page(json!({"items":[{}]})).is_err());
        let (account, _, http) = fixture();
        account.accept_token(token(true), true).unwrap();
        http.gets.lock().unwrap().push_back(Ok(json!({"items":[]})));
        assert!(account.playlists(None).unwrap().items.is_empty());
        assert!(account.playlists(Some("x".repeat(1025))).is_err());
    }
    #[test]
    fn unauthorized_api_request_refreshes_once_and_retries() {
        let (account, _, http) = fixture();
        account.accept_token(token(true), true).unwrap();
        http.gets
            .lock()
            .unwrap()
            .push_back(Err("YouTube rejected authorization; sign in again".into()));
        http.forms.lock().unwrap().push_back(Ok(token(false)));
        http.gets.lock().unwrap().push_back(Ok(
            json!({"items":[{"id":"channel", "snippet":{"title":"Test channel"}}]}),
        ));
        assert_eq!(account.channel().unwrap().title, "Test channel");
        assert_eq!(
            &*http.calls.lock().unwrap(),
            &["channels", "token", "channels"]
        );
    }
    #[test]
    fn unavailable_store_fails_closed_before_any_network_or_browser_login() {
        struct LockedStore;
        impl SecretStore for LockedStore {
            fn read(&self, _: &str) -> Result<Option<Zeroizing<String>>, String> {
                Err("locked".into())
            }
            fn write(&self, _: &str, _: &str) -> Result<(), String> {
                Err("locked".into())
            }
            fn delete(&self, _: &str) -> Result<(), String> {
                Err("locked".into())
            }
        }
        let http = Arc::new(FakeHttp::default());
        let account = Account::with(
            Some(config()),
            Box::new(LockedStore),
            Box::new(http.clone()),
        );
        assert!(account.prepare_client().is_err());
        assert!(account.sign_in().is_err());
        assert!(http.calls.lock().unwrap().is_empty());
    }
    #[test]
    fn http_errors_are_redacted_and_response_size_is_bounded() {
        fn response(status: &str, body: String) -> Response {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let status = status.to_string();
            thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let _ = read_callback(&mut stream);
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            });
            Client::builder()
                .no_proxy()
                .build()
                .unwrap()
                .get(format!("http://{address}/test"))
                .send()
                .unwrap()
        }
        let error = response_json(response(
            "400 Bad Request",
            json!({"error":"invalid_grant", "error_description":"SECRET_MUST_NOT_LEAK"})
                .to_string(),
        ))
        .unwrap_err();
        assert!(error.contains("revoked") && !error.contains("SECRET_MUST_NOT_LEAK"));
        assert!(
            response_json(response("200 OK", "a".repeat(MAX_BODY as usize + 1)))
                .unwrap_err()
                .contains("size limit")
        );
        assert_eq!(
            response_json(response("200 OK", String::new())).unwrap(),
            Value::Null
        );
    }
    #[test]
    fn missing_publisher_registration_never_uses_old_user_configuration() {
        let (_, store, http) = fixture();
        store.write("refresh", "old-alpha-grant").unwrap();
        let account = Account::with(None, Box::new(store), Box::new(http.clone()));
        let status = account.status().unwrap();
        assert!(!status.available && !status.has_session);
        assert!(account.sign_in().unwrap_err().contains("this build"));
        assert!(account.token().is_err());
        assert!(http.calls.lock().unwrap().is_empty());
    }
    #[test]
    fn repeated_sign_in_setup_preserves_a_grant_for_the_same_publisher() {
        let (account, store, _) = fixture();
        account.accept_token(token(true), true).unwrap();
        account.prepare_client().unwrap();
        assert_eq!(&**store.read("refresh").unwrap().unwrap(), "fake-refresh");
        assert!(account.status().unwrap().has_session);
        assert!(account.access.lock().unwrap().is_some());
    }
    #[test]
    fn restoring_a_saved_session_verifies_google_before_reusing_it() {
        let (account, store, http) = fixture();
        store.write("refresh", "saved-grant").unwrap();
        http.forms.lock().unwrap().push_back(Ok(token(false)));
        for _ in 0..2 {
            http.gets
                .lock()
                .unwrap()
                .push_back(Ok(music::tests::account_response()));
        }
        account.restore_session().unwrap();
        assert_eq!(
            &*http.calls.lock().unwrap(),
            &["token", "music:account/account_menu"]
        );
        account.restore_session().unwrap();
        assert_eq!(
            &*http.calls.lock().unwrap(),
            &[
                "token",
                "music:account/account_menu",
                "music:account/account_menu"
            ]
        );
    }
    #[test]
    fn another_publishers_grant_is_never_refreshed() {
        let (_, store, http) = fixture();
        store.write("refresh", "other-grant").unwrap();
        let changed = ClientConfig {
            client_id: "other.apps.googleusercontent.com".into(),
            client_secret: "other-secret".into(),
        };
        let account = Account::with(Some(changed), Box::new(store), Box::new(http.clone()));
        assert!(!account.status().unwrap().has_session);
        assert!(account.restore_session().is_err());
        assert!(http.calls.lock().unwrap().is_empty());
    }
    #[test]
    fn google_authorization_alone_does_not_verify_music_login() {
        let (account, store, http) = fixture();
        store.write("refresh", "saved-grant").unwrap();
        http.forms.lock().unwrap().push_back(Ok(token(false)));
        http.gets
            .lock()
            .unwrap()
            .push_back(Ok(serde_json::json!({"actions":[]})));
        assert!(account.restore_session().is_err());
    }
    #[test]
    fn music_search_and_resolution_refuse_anonymous_account_responses() {
        let (account, _, http) = fixture();
        account.accept_token(token(true), true).unwrap();
        for _ in 0..2 {
            http.gets
                .lock()
                .unwrap()
                .push_back(Ok(serde_json::json!({"actions":[]})));
        }
        assert!(account.search_music("song".into()).is_err());
        assert!(account.resolve_music_stream("abcdefghijk".into()).is_err());
        assert_eq!(
            &*http.calls.lock().unwrap(),
            &["music:account/account_menu", "music:account/account_menu"]
        );
    }
    #[test]
    fn music_api_retries_authorization_once_with_a_refreshed_token() {
        let (account, _, http) = fixture();
        account.accept_token(token(true), true).unwrap();
        http.gets
            .lock()
            .unwrap()
            .push_back(Err("YouTube rejected authorization; sign in again".into()));
        http.forms.lock().unwrap().push_back(Ok(token(false)));
        http.gets
            .lock()
            .unwrap()
            .push_back(Ok(music::tests::account_response()));
        http.gets
            .lock()
            .unwrap()
            .push_back(Ok(serde_json::json!({"contents":[]})));
        assert!(account.search_music("song".into()).unwrap().is_empty());
        assert_eq!(
            &*http.calls.lock().unwrap(),
            &[
                "music:account/account_menu",
                "token",
                "music:account/account_menu",
                "music:search"
            ]
        );
    }
    #[test]
    fn invalid_music_input_never_reaches_the_network() {
        let (account, _, http) = fixture();
        assert!(account.search_music(" ".into()).is_err());
        assert!(account.search_music("x".repeat(201)).is_err());
        assert!(account
            .resolve_music_stream("https://example.invalid".into())
            .is_err());
        assert!(http.calls.lock().unwrap().is_empty());
    }
    #[test]
    fn failed_music_verification_does_not_replace_a_previous_grant() {
        let (account, store, http) = fixture();
        account.accept_token(token(true), true).unwrap();
        let mut next = token(true);
        next["refresh_token"] = "unverified-refresh".into();
        next["access_token"] = "unverified-access".into();
        http.gets
            .lock()
            .unwrap()
            .push_back(Ok(serde_json::json!({"actions":[]})));
        assert!(account.finish_sign_in(next).is_err());
        assert_eq!(&**store.read("refresh").unwrap().unwrap(), "fake-refresh");
        assert_eq!(&*account.token().unwrap(), "fake-access");
    }
    #[test]
    fn cancelled_login_does_not_persist_new_credentials() {
        let (account, store, http) = fixture();
        account.cancel();
        assert!(account.finish_sign_in(token(true)).is_err());
        assert!(store.read("refresh").unwrap().is_none());
        assert!(http.calls.lock().unwrap().is_empty());
    }
    #[test]
    fn a_new_grant_is_saved_only_after_music_account_verification() {
        let (account, store, http) = fixture();
        http.gets
            .lock()
            .unwrap()
            .push_back(Ok(music::tests::account_response()));
        account.finish_sign_in(token(true)).unwrap();
        assert_eq!(&**store.read("refresh").unwrap().unwrap(), "fake-refresh");
        assert_eq!(
            &*http.calls.lock().unwrap(),
            &["music:account/account_menu"]
        );
    }
    #[test]
    fn native_session_is_verified_and_chunked_in_secure_store() {
        let (account, store, http) = fixture();
        http.gets
            .lock()
            .unwrap()
            .push_back(Ok(music::tests::account_response()));
        let cookie = format!("SAPISID=fixture; other={}", "x".repeat(5000));
        account.save_music_session(cookie.clone()).unwrap();
        assert_eq!(account.session_cookie().unwrap().unwrap().as_str(), cookie);
        assert!(store.0.lock().unwrap().values().all(|v| v.len() <= 1024));
        assert!(account.status().unwrap().has_session);
        account.forget_session().unwrap();
        assert!(account.session_cookie().unwrap().is_none());
    }
    #[test]
    fn session_search_uses_native_cookie_auth_without_oauth_registration() {
        let (_, store, http) = fixture();
        let account = Account::with(None, Box::new(store), Box::new(http.clone()));
        for response in [
            music::tests::account_response(),
            music::tests::account_response(),
            serde_json::json!({"contents":[]}),
        ] {
            http.gets.lock().unwrap().push_back(Ok(response));
        }
        account
            .save_music_session("SAPISID=fixture".into())
            .unwrap();
        assert!(account.search_music("song".into()).unwrap().is_empty());
        assert_eq!(
            &*http.calls.lock().unwrap(),
            &[
                "session:account/account_menu",
                "session:account/account_menu",
                "session:search"
            ]
        );
    }
    #[test]
    fn rejected_login_does_not_replace_an_existing_session() {
        let (account, _, http) = fixture();
        http.gets
            .lock()
            .unwrap()
            .push_back(Ok(music::tests::account_response()));
        account.save_music_session("SAPISID=first".into()).unwrap();
        http.gets
            .lock()
            .unwrap()
            .push_back(Ok(serde_json::json!({"actions":[]})));
        assert!(account
            .save_music_session("SAPISID=rejected".into())
            .is_err());
        assert_eq!(
            account.session_cookie().unwrap().unwrap().as_str(),
            "SAPISID=first"
        );
    }
    #[test]
    fn invalid_publisher_configuration_is_rejected() {
        let invalid = ClientConfig {
            client_id: "invalid".into(),
            client_secret: "invalid".into(),
        };
        assert!(Account::new(Some(invalid)).is_err());
    }
    #[test]
    fn busy_account_rejects_overlapping_operations() {
        let (account, _, _) = fixture();
        let _guard = account.operation.lock().unwrap();
        assert!(account.sign_in().is_err());
        assert!(account.channel().is_err());
        assert!(account.playlists(None).is_err());
    }
}
