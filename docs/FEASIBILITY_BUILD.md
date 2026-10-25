# Native Music session test build

This build uses kYuutube's own UI and native libmpv player. It does not open the
YouTube Music website as its playback interface. Alpha 3's portal mode is historical.

Choose **Sign in with Google**, complete the temporary login window, then choose
**Finish sign-in** in the native app. Rust verifies the Music account and saves
only this app's session in the OS keystore. No Google Cloud client or cookie export
is needed. Google may reject embedded login; no live Google acceptance is claimed.
See `GOOGLE_ACCOUNT.md` for account testing.

Windows requires WebView2; Linux requires WebKitGTK 4.1; macOS uses WKWebView.
Native audio currently also requires a trusted libmpv runtime; it is not bundled.
Accepted adjacent library names are `libmpv-2.dll` (Windows), `libmpv.2.dylib`
(macOS), and `libmpv.so.2` (Linux). If using a trusted library elsewhere, set
`KYUUTUBE_MPV_LIBRARY` to its absolute path. Do not use an untrusted DLL. This
runtime setup is unfinished development packaging, not the intended end-user flow.

Search for a song, select it, and check play/pause, seek, volume, and stop.
Report whether the account connected and whether a real track advanced audibly.
Never include cookies, passwords, authorization headers, or signed stream URLs.
The current resolver supports direct audio leases; cipher-only, protected, and
ad-requiring responses are refused. Riff's complete yt-dlp resolver is not ported.
Offline downloads and full feature parity remain unfinished.

These are unsigned development binaries, not production installers. Live account
and Music playback are unverified because this cloud proxy blocks the service.
Linux memory remains over budget. Local HTTP/WAV tests and successful compilation
do not certify real Music playback. See the repository's TEST_REPORT.md.
