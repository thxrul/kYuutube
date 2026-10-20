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
| Google login and token refresh | Not run; no client configuration/interactive consent |
| Audio playback | Generated local WAV through a virtual sink passed; physical audio and YouTube not run |
| API, queue, auth, resolution unit/integration tests | Not run; production implementation absent |
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
