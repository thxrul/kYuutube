# kYuutube

Independent desktop launcher for the official YouTube Music web player.
Not affiliated with Google or YouTube.

The new test mode needs no Google Cloud setup: **open → Open YouTube Music →
sign in on the official site**. It uses your OS webview in a separate private
window. If Google declines embedded sign-in, choose **Open in browser** to use
your normal browser and its existing account. In-app sessions are not persisted.
No passwords or cookies are extracted, no scripts are injected into the site,
and remote windows cannot invoke app commands.

This is an official-web-player test preview, not verified native Rust streaming
or full Metrolist parity. YouTube handles playback, ads, and subscriptions.
The cloud blocks YouTube Music, so live sign-in/playback still needs your testing.
The previous memory target remains unmet/unverified for this mode. Offline
YouTube downloads are not implemented. No telemetry, ad blocking, or Premium
bypass is implemented.

See [sign-in instructions](docs/GOOGLE_ACCOUNT.md) and the
[sign-in options review](docs/SIGN_IN_OPTIONS.md). Windows requires WebView2;
Linux requires WebKitGTK 4.1; macOS uses WKWebView. libmpv is unnecessary for this
mode. Downloads are unsigned standalone executables, not installers.

The experimental native OAuth/stream code remains in the repository for research.
Its previous tests do not establish that the new web-player login works. Alpha 3
is authorized for testing by the user's latest instruction; native playback must
still not be advertised as verified.
