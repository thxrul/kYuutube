# kYuutube account and local-audio diagnostic

This is an unsigned diagnostic, **not the complete YouTube Music application**.
It includes native system-browser Google OAuth for read-only YouTube channel and
playlist checks, plus a local-file audio player. YouTube Music streaming, offline
downloads, library sync, and Metrolist feature parity are not implemented.

For account sign-in, follow the included `GOOGLE_ACCOUNT.md`. The publisher
registers OAuth once; users do not enter client credentials. The account screen
needs WebView2 on Windows and an OS password store. It does not require libmpv
or a configured audio sample. No Google password is entered in the app.

Local audio tests are now under **About kYuutube → Audio diagnostics**. The
main screen contains only the account flow and a short playback-preview notice.

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

For a physical Windows measurement, start the visible window, then capture its
entire process tree (including WebView2 helpers) in a separate PowerShell prompt:

```powershell
$player = Get-Process -Name kyuutube-desktop-spike
.\Measure-ProcessTree.ps1 -RootProcessId $player.Id -Seconds 30 -OutputPath .\windows-idle.json
```

Measure idle and local playback in separate runs. The helper records names,
working-set RAM and CPU only, never tokens, command lines, or environment values.
Its RAM metric sums working sets and can double-count shared pages; it is not
PSS. CPU is a percentage of one core. The program must remain running throughout.
Run it from your PowerShell development environment.

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

CI artifacts are temporary, unsigned diagnostics. Windows/macOS runtime behavior remains unverified
unless TEST_REPORT.md records an actual test on those platforms.
