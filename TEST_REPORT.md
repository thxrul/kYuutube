# Test report

Date: 2026-10-21 (Asia/Colombo). Phase 0 only; native diagnostic checks passed, full application tests remain unrun.

| Check | Result |
| --- | --- |
| Git remote read and repository metadata | Passed; empty repository, metadata lists push permission |
| Install isolated research dependencies | Passed, including a fresh environment and dependency check |
| Rust/Tauri Linux release build | Passed |
| Native integration tests | 2 passed, 0 failed, 0 skipped |
| Clippy, native core/all targets, warnings as errors | Passed |
| Tauri/WebKitGTK UI diagnostic | Six checks completed, described below |
| Direct launch without WebDriver | Measured; memory budget failed |
| Local package SHA-256 verification | Passed |
| Anonymous music search | Failed: proxy denial |
| Stream metadata resolution, no media download | Failed: proxy denial |
| Independent YouTube / YouTube Music HTTPS checks | CONNECT HTTP 403; diagnostics identify policy denial |
| Live Google login and token refresh | Not run; no client configuration/interactive consent; local protocol tests are recorded below |
| Audio playback | Generated local WAV through a virtual sink passed; physical audio and YouTube not run |
| Production Music API, queue, stream resolution tests | Not run; production implementation absent |
| Full application UI end-to-end tests | Not run; only local diagnostic UI exercised |
| Three full bug-hunt passes | Not run; no fake clean passes recorded |
| Failure injection | Not run |
| Two-hour playback soak | Not run |
| Windows/Linux/macOS throttled playback | Not run |
| Desktop/minimum OS, device changes, sleep/wake, HiDPI/multi-monitor | Not run |

No full application implementation exists. The diagnostic is not a production client.
The observed live probe failures are environment access failures, not evidence
that upstream search or playback is broken. Full application completion and
release remain blocked; the report must be updated with actual run evidence.

## Native and UI evidence

The two native tests cover URL/missing-file/directory rejection, advancing local
WAV decode time, pause stability, seek completion, volume/seek bounds including
nonfinite values, resume, and invalid-command errors. Rust's zero-test doctest
phase contributes no validation count.

Actual Tauri/WebKitGTK automation verified UI loading, Play sample timeline
advancement, pause stability, seeking to five seconds, submission of a volume UI
event, and no horizontal overflow at 360×480 and 1920×1080. Hardware loudness,
full keyboard/screen-reader behavior, and physical display/device changes remain
unverified. Screenshot: `reports/desktop-spike.png`.

## Setup and spike iterations

1. Host apt could not write the system database. Resolved with an isolated native
   development container and verified dependencies.
2. D-Bus rejected an unnamed UID. Added the matching user to the image.
3. WebDriver rejected an unsupported `--host` option. Removed it.
4. A fixed two-second wait inspected playback before async loading completed.
   Replaced it with a bounded wait for actual progress. The assertions then passed;
   none was disabled or changed to a skip.
5. Direct launch confirmed the high memory was not primarily WebDriver overhead.
   The default configuration fails the memory budget; no performance pass is claimed.

These are targeted setup/spike iterations, not three complete application
bug-hunt passes. Windows/macOS CI compilation must be checked separately and does
not establish runtime or minimum-OS compatibility. Real playback, failure
injection, soak, throttle, Wayland, and physical hardware tests remain outstanding.

## Graphics comparisons and native CI

After user approval, the same six actual Tauri UI assertions passed with WebKit's
Never policy, and again with Never plus its prelaunch compositing switch. Both
cases retain failing memory measurements in `PERFORMANCE.md`. The comparisons are
not three complete bug-hunt passes of the full music application.

Baseline workflow run 39500201138 compiled all four platform targets successfully.
Linux CI executed 2 native integration tests, both passing. All four downloaded
artifacts passed their own SHA-256 manifests and binary architecture inspection.
Windows/macOS runtime tests, real audio devices, and minimum-OS compatibility are
still unrun. The updated Windows measurement helper has a separate CI validation;
its result must be checked before calling that helper verified on Windows.

## Verified updated diagnostic artifacts

Workflow **39502246120**, source commit `508ecab`, passed compilation for Linux
x64, Windows x64, Intel macOS, and Apple Silicon macOS. Linux executed 2 native
tests, both passing. Windows executed the process-tree helper for five seconds
against its shell and a real child, requiring positive RAM and at least two
processes. That validates the helper, not Windows GUI/audio.

All 28 entries across the four artifact SHA-256 manifests verified byte-for-byte.
Both executables in every artifact had the selected PE, ELF, or Mach-O architecture.
The helper text matched source after normalizing Windows checkout line endings;
no artifact or expected checksum was modified. `reports/ci-builds.json` records
artifact links, digests, and binary sizes. The full client remains incomplete.

## Native Google account iteration (2026-10-21)

The user confirmed system-browser OAuth, requested offline downloads, and excluded
Last.fm/Discord. Metrolist's Android cookie login was inspected but not copied.
The new Rust account diagnostic implements desktop OAuth and read-only YouTube
Data API checks. It is not verified YouTube Music authentication or playback.

- `cargo test --locked -p kyuutube-youtube-account`: **13 passed, 0 failed**.
  Tests use fictitious credentials and in-memory test stores; they include a real
  local TCP callback and HTTP response parsing. Coverage includes RFC 7636 PKCE,
  fresh random state/verifier, callback host/path/state/duplicate rejection,
  cancellation/timeout, bounded responses and redacted errors, refresh lifecycle,
  transient failure versus revoked grant, one retry after an API 401, configuration
  changes, unavailable secure store, disconnect while offline, bounded playlist
  pages, and overlapping account operations. No real Google tokens were used.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: passed.
- `cargo build --release --locked --workspace`: passed on Linux.
- Real Tauri/WebKitGTK/Xvfb UI run with the previously tested software graphics
  settings: **7 checks passed**. The new check submitted an invalid client through
  native IPC, verified a validation error, checked that the password field was
  cleared, and checked there were no localStorage credentials. Existing local
  play/pause/seek/volume and compact/wide layout checks passed.
- Evidence: `reports/account-desktop.json`, `reports/account-desktop.png`.
  Local playback measured median **473.07 MiB summed RSS / 277.40 MiB PSS**,
  **1.91%** of one CPU core during the short automated measurement. Idle was
  **429.11 MiB RSS / 235.78 MiB PSS**, **0.42%** CPU. Memory still fails.

Initial integration found a missing explicit reqwest query feature; enabling it
fixed compilation. Review also added a total callback-read deadline and a single
refresh retry after API authorization rejection, with tests. These are targeted
account iterations, not three full application bug-hunt passes.

Live system-browser consent, actual OS-keystore persistence on each platform,
real Music metadata/playback/downloads, physical audio, soak, throttle, and full
Metrolist parity tests remain unrun. Windows consent needs the user's own Desktop
app client and interactive browser. See `docs/GOOGLE_ACCOUNT.md` for safe steps.


The account source at `14cc092589f06412ae66353aff304f97fc40e515` passed
[four-platform build run 39567857094](https://github.com/thxrul/kYuutube/actions/runs/39567857094).
Thirteen account tests executed and passed on each of Windows x64, Linux x64,
and macOS Apple Silicon. Intel macOS was cross-compiled and did not execute the
tests. Linux's two native audio tests passed, and Windows's process-tree helper
check passed. All nine manifest entries per target were verified after download,
and the desktop executable architectures matched their labels. This does not
validate Google consent, platform keychain persistence, or Windows/macOS GUI/audio.

[Alpha 2](https://github.com/thxrul/kYuutube/releases/tag/v0.0.1-alpha.2) was
published as an unsigned diagnostic prerelease from that exact source commit.
The publication workflow downloaded all public release assets and verified the
four archive SHA-256 hashes. See `reports/account-ci-builds.json`. Full Music
login/playback, offline downloads, final audits, and production release gates
remain outstanding.
