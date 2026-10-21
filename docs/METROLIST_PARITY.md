# Metrolist desktop parity investigation

Reference: [Metrolist at b1dd1d0](https://github.com/metrolistgroup/metrolist/tree/b1dd1d06722171f122ed73d7f57c127784bd959d), reviewed 2026-10-21. This inventory covers its README and the account, playback, library, discovery, podcast, recognition, settings, and social source areas. The reference is an Android application; its APK and Jetpack Compose/Media3 implementation cannot be compiled into a Windows, macOS, or Linux application.

## User scope

The user requests native YouTube login and Metrolist's functions on desktop. Their subsequent clarification explicitly adds **offline downloads**, superseding the original brief's no-download requirement, and excludes **Last.fm and Discord**. The earlier no-ad-skipping/no-Premium-bypass requirements, OS-keystore rule, and 200 MB / 10% CPU target have not been waived. Third-party services remain outside the original Google/YouTube-only network scope unless separately agreed; this affects lyrics translation, recognition, and listen-together.

## Authentication finding

Metrolist's `LoginScreen.kt` opens Google sign-in in Android WebView, reads `CookieManager` for the YouTube Music origin, and extracts visitor/session information from the signed-in page. Its InnerTube implementation then uses that browser session. This is not Google's desktop OAuth flow.

The requested desktop flow uses the system browser, PKCE S256, a random state, a short-lived loopback callback, and Windows Credential Manager / macOS Keychain / Linux Secret Service. The current account flow uses a publisher-owned **Desktop app** OAuth client embedded by the release build. End users see one sign-in button and complete consent in their system browser; they do not register a Google Cloud project. It can attempt read-only YouTube Data API channel and playlist access. It **does not create or import a YouTube Music browser session** and must not be described as Metrolist-compatible login. No Google password or browser-cookie import is requested.

The existing cloud proxy still returns CONNECT 403 for `music.youtube.com`. A successful account-consent test needs the user's Windows browser and client configuration. A successful Data API test will still not validate InnerTube, stream resolution, entitlements, downloads, or playback. Do not add a fake connected state or treat compilation and mocked-token tests as live account validation.

## Feature inventory and implementation status

| Metrolist function | Desktop status / next requirement |
| --- | --- |
| Google / YouTube Music login, channel switching | Publisher-configured browser OAuth plus Music account verification implemented experimentally; live consent/Music compatibility unverified; channel switching unavailable |
| Personalized home / quick picks / recommendations | Pending a verified Music account API |
| Search songs, albums, artists, videos, playlists | Experimental native Music search implemented with bounded results; live access unverified |
| Album / artist detail pages, subscriptions | Not implemented |
| Charts, moods, genres, new releases, related music | Not implemented |
| YouTube Music library, likes, saved albums/artists, bidirectional sync | Not implemented; Data API read-only playlist diagnostic is not this feature |
| Remote playlists: create, rename, delete, import, add/remove/reorder tracks | Not implemented; secure OAuth scope and Music/YouTube playlist compatibility need validation |
| Local playlists and queue, reorder, shuffle/repeat, session resume | Not implemented |
| Network music/video and background playback | Experimental Music resolver and native network playback implemented; loopback HTTP audio tested; real Music playback unverified |
| Play/pause, seek, volume | Implemented for local files and native network audio; real Music track and physical output remain unverified |
| Offline downloads, cache, quota/storage management | Requested and in scope; not implemented. Requires a verified authenticated media path and account entitlement handling |
| Gapless/crossfade, normalization, silence skipping | Not implemented |
| Equalizer, tempo, pitch, audio quality | Not implemented |
| Sleep timer / alarm | Not implemented; mobile alarm behavior needs a desktop equivalent |
| Synced lyrics, translation, romanization | Not implemented; external providers conflict with the current endpoint restriction |
| Podcasts: browse, subscribe, save, episodes, progress | Not implemented |
| Uploaded music: listing, upload, deletion | Not implemented |
| History, listening stats, wrapped | Not implemented; no telemetry is enabled |
| Recognition and recognition history | Not implemented; reference relies on third-party recognition services |
| Listen together / rooms / shared queues | Not implemented; reference requires a separate backend beyond Google/YouTube |
| Casting | Not implemented; optional Android Cast support is not portable |
| Backup / restore / local settings / updates | Not implemented |
| Themes: light/dark/black/dynamic, palettes, contrast | Existing static token-based dark diagnostic only; full theming unfinished |
| Material 3 adaptive navigation, now-playing view | Diagnostic shell only; not feature-complete |
| Android home widget, Quick Settings tile, Android Auto | Android-specific; desktop equivalents would be tray/menu bar, media keys, OS Now Playing/MPRIS/SMTC, desktop shortcuts |
| Last.fm and Discord | Excluded by the user |

## Delivery gates

1. Build and test the secure account diagnostic, including invalid callbacks, expiry, cancellation, revoked credentials, bounded API responses, and unavailable keystore behavior.
2. Validate system-browser consent on Windows with the user's own Desktop client; do not send secrets or tokens in chat.
3. Prove an authenticated YouTube Music metadata/playback path that preserves ads and entitlements. Decide whether the browser-session approach is compatible with the system-browser-only login requirement before porting it.
4. Resolve the failed Tauri/Linux memory budget with measured physical-platform evidence or a separately accepted architecture/budget change. The last local playback measurement was 271.74 MiB PSS / 468.77 MiB summed RSS; it remains a failure.
5. Implement the remaining feature groups against those verified interfaces, then run the required full test passes, soak, platform/throttle checks, and design audit before claiming parity.

The code added in this iteration is original Rust and JavaScript; no Metrolist implementation, artwork, or branding was copied. Metrolist's GPL-3.0 license must be addressed if future implementation copies or adapts its source. This document is a feature analysis, not a claim of an approved or finished port.


The user subsequently made **working native Music playback before publishing**
an explicit condition. New release packaging now refuses unverified playback
artifacts. The experimental service adapter uses a single fixed Music client,
requires an authenticated account-menu response, and does not fall back to cookie
imports, alternate clients, cipher transforms, attestation providers, or ignored
advertisements. This is implementation progress, not a live playback pass.
