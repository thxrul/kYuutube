# Sign-in alternatives

Reviewed Metrolist's Android WebView login implementation, Google installed-app OAuth guidance, and Tauri's system-webview/incognito/navigation APIs.

| Approach | Setup | Limitation |
| --- | --- | --- |
| Publisher desktop OAuth | One registration and public consent configuration | Does not prove Music stream compatibility |
| Metrolist-style cookie extraction | In-app login without Cloud setup | Sensitive session extraction; Google can reject embedded sign-in |
| Official Music site in isolated OS webview | No OAuth registration or extraction | Embedded login may be rejected; web-player resource cost |
| Official Music site in normal browser | No app registration; existing account can be reused | Playback is in the browser |

Alpha 3 selects the official-site webview with an explicit normal-browser fallback. It does not intercept passwords, read cookies, inject scripts, spoof the browser, or export sessions. The remote window has no remote IPC capabilities and top-level navigation is restricted to specific HTTPS Google/YouTube login and Music origins. In-app storage is ephemeral to avoid plaintext persistent session data on Linux; the normal browser manages its own saved login.

Google's acceptance of a system webview cannot be guaranteed. The normal-browser option uses a supported browser directly instead of attempting to evade a rejection. Official-site playback handles ads and account restrictions. Native stream extraction remains experimental and is not used by the launcher.

The user explicitly allowed a different sign-in method and requested compiled test binaries; alpha 3 supersedes the previous publication hold for this clearly labeled web-player test mode only. Native Rust playback still cannot be described as verified.

References:
- https://github.com/metrolistgroup/metrolist/blob/b1dd1d06722171f122ed73d7f57c127784bd959d/app/src/main/kotlin/com/metrolist/music/ui/screens/LoginScreen.kt
- https://developers.google.com/identity/protocols/oauth2/native-app
- https://docs.rs/tauri/2.12.4/tauri/webview/struct.WebviewWindowBuilder.html


The subsequent Riff-guided native request replaces the portal as the default. The
current login-only window supplies a verified account session to Rust; playback
is in the native player. Alpha 3 remains historical, not the current interface.
