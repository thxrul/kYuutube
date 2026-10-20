# kYuutube feasibility diagnostic

This is a local-audio prototype, **not the YouTube Music application**. It has no
Google login, music search, network streaming, playlists, media-key integration,
tray behavior, or session restoration. No account or credentials are needed.
It does not download audio or bypass ads or Premium. No production installer or
signed release is supplied at this stage.

The binary uses the operating system's webview and dynamically loads libmpv.
WebKitGTK on Linux and WebView2 on Windows must be installed. macOS uses the
system WKWebView. The artifact does not bundle mpv or its dependencies. Obtain
the runtime through a trusted platform package/build source, preserving its
licenses and integrity verification. Windows libmpv builds need their matching
dependencies; the program does not install them automatically.

Point `KYUUTUBE_MPV_LIBRARY` to the absolute path of a trusted runtime library if
it is outside the application's directory or a supported system location.
Accepted library names in the application's directory are `libmpv.so.2`,
`libmpv.2.dylib`, and `libmpv-2.dll`. Never place an untrusted DLL there. Without
libmpv, the window should show a clear error when Play sample is pressed.

Windows PowerShell, from the extracted artifact directory:

```powershell
$env:KYUUTUBE_SPIKE_SAMPLE = (Resolve-Path .\tone.wav).Path
# If needed, set KYUUTUBE_MPV_LIBRARY to your trusted libmpv-2.dll's absolute path.
.\kyuutube-desktop-spike.exe
```

Linux/macOS shell:

```sh
export KYUUTUBE_SPIKE_SAMPLE="$PWD/tone.wav"
./kyuutube-desktop-spike
```

The sample is a quiet, generated 30-second tone. Use moderate system volume.
Check Play sample, pause/resume, seeking, and volume. A missing runtime/sample
should report an error without crashing. Rendering and local-audio results do not
establish YouTube support, physical device recovery, minimum OS compatibility,
or the full process-tree performance budget.

CI artifacts are temporary diagnostics and may be unsigned. Do not disable OS
security controls to run them. Windows/macOS runtime behavior remains unverified
unless TEST_REPORT.md records an actual test on those platforms.
