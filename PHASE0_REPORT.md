# Phase 0: feasibility investigation

Date: 2026-10-21 (Asia/Colombo). Status: **local native spike tested; YouTube path blocked; Linux webview budget failed**.

## Decision

Do not proceed to a production implementation or release yet. The user approved
continuing the conditional Tauri + Rust feasibility spike. A local-file native
audio engine and a Tauri window now compile and pass their targeted checks, but
the requested Google login → search → YouTube playback path remains unproven.
The Linux virtual-display shell exceeds the memory budget even before the audio
engine loads. This is a failed budget check for this configuration, not proof
that every physical desktop or every architecture must fail. No installer or
production application release has been built.

The user's direct request is to support Windows testing and eventually publish
compiled application files on GitHub. The uploaded brief supplies the proposed
features, constraints, and the Phase 0 / architecture-sign-off sequence; the
Material 3 attachment supplies design guidance. Neither attachment is an installed
tool or a source of permissions. The Windows test offer does not waive the
brief's Linux/macOS validation requirements.

## Findings from current primary sources

### Authentication

Google documents device-code OAuth for TVs and devices with limited input. Its
desktop guidance instead uses the system browser, a local loopback redirect,
state validation, and PKCE. An installed client cannot keep an embedded client
secret confidential. Choose the desktop flow for a desktop product, subject to
verifying that the unofficial metadata service accepts its tokens. That
compatibility has **not** been tested here.

ytmusicapi's current setup instructions still document a user-owned Google Cloud
client ID and secret of type **TVs and Limited Input devices**. This is evidence
of a library's implemented flow, not evidence that it is Google's recommended
desktop integration. Its example writes `oauth.json`; the product must not use
that plaintext storage pattern. Any persistent tokens and a user-supplied secret
must use Windows Credential Manager, macOS Keychain, or Linux Secret Service.
Fail closed if a secure store is unavailable; never fall back to a JSON file.

The machine has no `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`, `YTM_CLIENT_ID`, or
`YTM_CLIENT_SECRET` variables and no configured Google secret bindings. No Google
login was attempted and no user tokens were created. Values must not be sent in
chat. A first-run system-browser consent step on the user's computer is still
needed even after a client is configured.

### Playback and proof-of-origin

YouTube Music has no official public API. ytmusicapi provides unofficial metadata
operations, not an end-to-end licensed native music playback service.

The current yt-dlp YouTube extractor documentation explicitly says OAuth login
no longer works with its playback extraction. Its proof-of-origin guide describes
client-dependent requirements for player, media-server, and subtitle requests;
some tokens bind to a video and user session, expire, and can require a provider
with browser/attestation machinery. Therefore OAuth metadata access must never be
presented as proof that native playback works. Such machinery can also add
processes, network dependencies, update burden, and memory usage. No attestation
provider, cookie importer, or alternate-client workaround was installed here.

Selecting a direct media URL does not establish that YouTube's ads, entitlement
checks, or free-account behavior are preserved. This brief explicitly forbids
ad skipping and Premium bypass; a release requires evidence of compliant behavior
for the supported account types. No supported path has been demonstrated. Google
also documents restrictions on separating audio and altering its official player;
those terms need to be considered before choosing playback integration.

### Official-player fallback

A system-webview wrapper around the official player keeps player behavior with
the service, but remains an unproven fallback. System-browser OAuth does not
automatically sign a separate WebKit/WebView2 cookie jar into YouTube Music.
Embedded Google sign-in must not be substituted, and browser cookies must not be
scraped as a workaround. Codec availability, background playback entitlements,
service-origin requests, and full helper-process memory also need validation.
An external-browser handoff can use the user's existing browser session but
changes the requested in-app playback and resource-accounting contract.

## Executed spike

Installed isolated research tools outside the checkout using verified HTTPS:
Python 3.12.14, ytmusicapi 1.12.3, yt-dlp 2026.8.19, and psutil 7.2.2.
`tools/phase0_probe.py` executes an anonymous music search and a metadata-only
stream-resolution probe for a public sample video. The latter is not a music
entitlement test. It does not download media, play audio, import cookies, or
persist provider responses or stream URLs.

| Probe | Outcome | Wall seconds | Sampled peak RSS (MiB) | Sampled CPU seconds |
| --- | --- | ---: | ---: | ---: |
| Anonymous music search | Failed: ProxyError | 0.202 | 23.54 | 0.13 |
| Stream metadata resolution | Failed: DownloadError | 0.303 | 39.19 | 0.24 |
| System-browser login | Not run: client configuration and interactive consent absent | — | — | — |
| One-track playback | Not run | — | — | — |

Raw sanitized results: `reports/phase0-probes.json`. These short Python failure
paths are **not application or playback performance numbers**. CPU percentages
in that JSON describe startup/failure work, not steady-state CPU. RSS sampling
every 50 ms may miss short peaks; CPU accounting may miss work between the last
sample and process exit. No performance claim follows from these figures.

Separate `HEAD`/`GET` checks reproduced proxy CONNECT HTTP 403 for both
`music.youtube.com` and `www.youtube.com`. Current environment diagnostics identify
`policy_denied` and `domain is blocked`, despite reporting unrestricted HTTP
policy. This is an environment access restriction, not a YouTube API response or
proof that a new Google credential would fix access. `youtubei.googleapis.com`
returned HTTP 405 to HEAD, which demonstrates a reachable endpoint but not usable
API authorization. No route, alternate hostname, or proxy bypass was attempted.

## Native local-file spike after user sign-off

Implemented a small Rust/libmpv engine, a Tauri v2 local UI, and a WebDriver
functional probe. It has no Google auth, metadata API integration, stream
resolver, playlists, queue, media-key/tray integration, or persistent session.
The font is a locally bundled Roboto Flex Latin subset, with its OFL license and
verified npm artifact integrity recorded in `web/fonts/`.

The host account cannot write the system apt database. Native dependencies were
therefore installed inside a disposable Docker development image, retaining apt
signature and TLS/checksum verification. The base Rust image is digest-pinned;
Rust dependencies are recorded in `Cargo.lock`. No privileged container or host
networking, renderer-sandbox override, or TLS bypass was needed.

The native integration suite executed **2 tests: 2 passed, 0 failed**. It tests
local-file validation, advancing decoder time, pause stability, seek completion,
volume bounds including NaN/infinity, and native command errors. A separate
three-second null-output CLI run advanced normally. Clippy passed with warnings
as errors. These are spike checks, not the full application's required suites.

The real Tauri/WebKitGTK WebDriver run passed six checks: UI loaded, local play
advanced, pause stopped the timeline, seeking moved the timeline, a volume UI
event was submitted, and 360×480 / 1920×1080 windows had no horizontal overflow.
Hardware loudness was not checked. The generated sample played through a
PulseAudio null sink; no physical audio or real music stream was tested.

| Configuration | Median summed RSS (MiB) | Median summed PSS (MiB) | Average CPU (% of one core) | Sample interval |
| --- | ---: | ---: | ---: | --- |
| Native local-file engine, virtual audio sink | 67.08 | 63.40 | 0.61 | 12 seconds |
| Direct-launch desktop idle, no WebDriver | 509.49 | 310.28 | 0.07 | 15 seconds |
| WebDriver desktop idle | 509.66 | 308.66 | 10.36 | 10 seconds |
| WebDriver desktop, local-file playback | 564.45 | 363.15 | 23.98 | 15 seconds |

These are **short Linux x86_64 / Xvfb measurements**, not normal physical-desktop,
Windows, macOS, YouTube, two-hour soak, or throttled test results. The virtual
display uses software graphics; WebDriver instrumentation also affects CPU. The
direct launch rules out WebDriver as the main source of this memory cost, while
its lower idle CPU shows why automation CPU cannot be treated as production idle
CPU. Playback CPU without WebDriver has not been measured.

A representative idle tree contained the application (~166 MiB RSS),
WebKitNetworkProcess (~68 MiB), and WebKitWebProcess (~276 MiB). The metric sums
RSS, which counts shared pages more than once; PSS apportions shared pages.
**Both** exceed the roughly 200 MB budget here. The result must not be relabeled
a pass by switching metrics. Xvfb, PulseAudio, the Python monitor, D-Bus, and
WebDriver itself are test infrastructure and are excluded from the app tree.
Samples every 500 ms can miss short-lived processes and peaks.

Stripped executables from the local release build: about **6.2 MiB desktop** and
**410 KiB audio CLI**. These are executable sizes, excluding system webview,
libmpv and its dependencies, and are not installer or installed-footprint sizes.

Evidence: `reports/audio-virtual-sink.json`, `reports/desktop-direct.json`,
`reports/desktop-webdriver.json`, and `reports/desktop-spike.png`. An earlier
anonymous-probe failure and the initial WebDriver startup attempt remain distinct
from the successful local-file run.

The development/test setup exposed and corrected three issues: a container UID
without a passwd entry prevented D-Bus startup; an unsupported driver CLI flag
prevented automation startup; and a fixed two-second delay inspected playback
before asynchronous file loading completed. The latter was replaced with a
bounded wait for real timeline progress, retaining the playback assertion. These
are targeted setup/spike iterations, **not** the brief's three full application
bug-hunt passes.

### Budget decision

The current Tauri/WebKitGTK configuration does not meet the memory target. Stop
before building the full client, as requested by the brief. Native audio alone
is inexpensive in this test, but changing the decoder alone cannot fix a shell
that exceeds the limit before the decoder loads.

The next useful comparisons are a physical-GPU Windows run of this diagnostic
and a supported WebKit hardware-acceleration policy experiment on Linux; WebKit
warns that disabling acceleration can increase CPU or affect rendering. Neither
is a claimed fix. A native toolkit such as Slint is a possible larger cut, but
would change the requested web/CSS design implementation and requires a scope
choice. The official-web-player fallback has not been measured and does not
remove the webview cost. No performance limit has been relaxed.

## Proposed architecture, conditional on the next spike

**Candidate:** Tauri v2, vanilla TypeScript, a local Material 3 token-based UI,
and a Rust backend with strict IPC permissions. Use system-browser desktop OAuth
with PKCE if the metadata compatibility test succeeds. Keep tokens in OS secure
storage. A native player remains conditional on a working, entitlement-respecting
stream integration. This is a recommendation for the next experiment, not a
signed-off implementation choice.

| Audio engine | Reason to evaluate | Unresolved cost |
| --- | --- | --- |
| libmpv | Mature network playback, seeking, buffering, and device handling | Native distribution/licensing and FFmpeg dependency footprint; cannot claim smaller installers or lower RSS without measuring |
| Symphonia + CPAL | Selective decoder features and native WASAPI/CoreAudio/ALSA output | HTTP range/reconnect, resampling, buffering, gapless behavior, device recovery, codec coverage and Opus integration need engineering and measurement |

Evaluate libmpv first for the spike because it reduces playback implementation
unknowns. Do not select the final engine on an unmeasured size assumption.
Symphonia's current upstream table lists Opus as an external integration; a
default feature set must not be assumed to decode every YouTube format.

**Fallback candidate:** official player in the system webview, only after proving
browser-only login can be reconciled without credential/cookie extraction. If it
cannot, an external-browser companion is a separate scope decision. Neither
fallback is claimed to fit 200 MB or the CPU budget.

## Platform validation and release plan

The outer executor is Linux x86_64 with no physical desktop or audio device.
Rust, libmpv, WebKitGTK, Xvfb, virtual audio, and WebDriver are now prepared in the
development container. Run them through `tools/native.sh`, not the host shell.
The prepared Docker/Xvfb session covers the targeted UI automation but cannot certify physical audio,
Wayland behavior, multi-monitor, HiDPI, sleep/wake, or real desktop resource use.

After sign-off, build on native GitHub Actions runners: Windows for MSI/NSIS,
macOS for DMG/app artifacts (Intel and Apple Silicon as selected), and Ubuntu for
Debian/AppImage packages. Pin dependencies and runner/toolchain choices, test the
declared minimum OS versions, retain checksums, and attach outputs to a versioned
GitHub Release only after required tests pass. A successful build alone does not
certify runtime behavior. Apple signing/notarization and Windows signing need
separate signing access; unsigned test binaries must be labeled accordingly.

The supplied GitHub integration can read repository metadata and reports push
permission. Reading Actions permission settings returned HTTP 403 (`Resource not
accessible by integration`); this alone does not establish that workflow runs or
artifact access are unavailable. Those specific operations remain untested.

The user can test Windows. Hosted macOS build/test jobs can provide automation,
but physical macOS behavior and throttled playback still require an interactive
Mac, a suitable remote Mac test session, or a tester. Do not mark them passed from
Linux. Keep three complete bug-hunt passes, failure injection, a two-hour real
playback soak, throttled tests, and the MD3 audit as explicit release requirements.

## Required next decisions and prerequisites

1. The conditional Tauri/native feasibility continuation was approved and
   executed. Decide the next budget investigation (physical Windows / graphics
   policy) or a native-toolkit scope change before building the full client. No
   finished architecture meets every requirement on present evidence.
2. Obtain supported YouTube access in a test execution environment. The current
   proxy denial persists under unrestricted settings; adding OAuth secrets will
   not repair it. Do not repeatedly broaden network settings or bypass policy.
3. Configure the appropriate user-owned Google Cloud OAuth client securely and
   complete interactive system-browser consent; first confirm which OAuth flow
   the chosen metadata integration actually supports.
4. Complete the login/search/play spike and measure the entire visible process
   tree before accepting the architecture. Preserve the roughly 200 MB / 10%
   budget; stop and report if it is exceeded rather than relaxing it silently.

## Sources inspected

Retrieved during this investigation; upstream projects may change:

- Google desktop OAuth: https://developers.google.com/identity/protocols/oauth2/native-app
- Google device OAuth: https://developers.google.com/identity/protocols/oauth2/limited-input-device
- ytmusicapi OAuth instructions: https://github.com/sigma67/ytmusicapi/blob/main/docs/source/setup/oauth.rst
- ytmusicapi FAQ: https://github.com/sigma67/ytmusicapi/blob/main/docs/source/faq.rst
- yt-dlp YouTube/OAuth notes: https://github.com/yt-dlp/yt-dlp/wiki/Extractors#youtube
- yt-dlp PO-token guide: https://github.com/yt-dlp/yt-dlp/wiki/PO-Token-Guide
- YouTube developer policies: https://developers.google.com/youtube/terms/developer-policies
- Tauri prerequisites: https://v2.tauri.app/start/prerequisites/
- libmpv project: https://github.com/mpv-player/mpv
- Symphonia codec/features table: https://github.com/pdeljanov/Symphonia
- CPAL platforms/prerequisites: https://github.com/RustAudio/cpal
- WebKit acceleration policy: https://github.com/WebKit/WebKit/blob/main/Source/WebKit/UIProcess/API/glib/WebKitSettings.cpp

The rendered ytmusicapi documentation returned HTTP 403; its upstream GitHub
documentation sources were read instead. The initial raw wiki PO-token URL
returned 404; the published GitHub wiki page was read successfully.
