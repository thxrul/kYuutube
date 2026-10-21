# Performance evidence

A local-audio diagnostic has been built. It fails the memory target on the tested
virtual Linux setup. No real YouTube playback measurements or installer sizes exist. The target remains about 200 MB for the full process tree
and 10% of one CPU core during normal visible playback/browsing. It has not been
relaxed or declared achievable.

| OS | Idle RAM/CPU | Playing RAM/CPU | Throttled RAM/CPU | Installer size |
| --- | --- | --- | --- | --- |
| Windows 10+ | Not measured | Not measured | Not measured | Not built |
| macOS 12+ | Not measured | Not measured | Not measured | Not built |
| Linux X11 | Not measured | Not measured | Not measured | Not built |
| Linux Wayland | Not measured | Not measured | Not measured | Not built |

Measured Linux x86_64 results (MiB; CPU as a percentage of one core):

| Configuration | Median RSS sum | Median PSS sum | CPU | Duration |
| --- | ---: | ---: | ---: | --- |
| Native local WAV → virtual audio sink | 67.08 | 63.40 | 0.61% | 12 s |
| Direct-launch Tauri idle, no WebDriver | 509.49 | 310.28 | 0.07% | 15 s |
| Tauri idle under WebDriver | 509.66 | 308.66 | 10.36% | 10 s |
| Tauri local playback under WebDriver | 564.45 | 363.15 | 23.98% | 15 s |

Rust 1.92.0, Tauri 2.12.4, WebKitGTK 2.50.6, libmpv 0.35.1-4, Debian Bookworm
container, Xvfb software graphics, and a PulseAudio null sink. These are short
virtual-display tests, not physical desktop, physical audio, real YouTube, soak,
or throttled results. Both RSS and PSS exceed the budget. RSS double-counts shared
pages; PSS apportions them. The direct launch rules out WebDriver as the primary
memory cost. Its lower CPU also shows why automation CPU cannot be treated as
normal idle CPU. Uninstrumented playback CPU remains unmeasured.

A representative idle tree: app 166 MiB RSS, WebKitNetworkProcess 68 MiB,
WebKitWebProcess 276 MiB. The application descendant tree is sampled every 500 ms;
short-lived processes/peaks may be missed. Xvfb, PulseAudio, D-Bus, Python, and
WebDriver are test infrastructure and excluded from this tree. No sandbox override
was needed. Changing only the decoder cannot fix a shell already above budget.

The stripped local desktop executable is about 6.2 MiB; the audio CLI is 410 KiB.
These exclude system WebKitGTK, libmpv and its dependencies and are neither
installer sizes nor total installed footprints. No production installer exists.

Evidence: `reports/audio-virtual-sink.json`, `reports/desktop-direct.json`, and
`reports/desktop-webdriver.json`. Earlier measurements are retained separately.
Physical-GPU and supported WebKit acceleration-policy comparisons, or a native
UI scope change, are possible next investigations; none is a claimed fix.

The Python probe failure-path measurements in `reports/phase0-probes.json` are
research-tool measurements, not entries for this table. They cannot select the
audio engine, establish a webview memory cost, or establish a playback budget.

Future measurements must include the application, webview helpers, decoder/player,
and any attestation helper processes. Report per-process RSS and a clearly named
aggregate (and PSS where available), CPU as a percentage of one core, installation
bytes including bundled dependencies, run duration, OS/hardware/tool versions,
sample interval, and foreground/hidden state. Do not confuse compressed installer
size with installed size or treat shared-page RSS sums as unique physical memory.

The required two-hour playback chart cannot be produced before real playback
works; no synthetic or fabricated chart is included.

## Approved graphics-policy comparisons

The original default-renderer measurements above are retained. Neither of these
opt-in comparisons meets the memory target:

| Linux configuration | Median RSS, MiB | Median PSS, MiB | CPU (% one core) |
| --- | ---: | ---: | ---: |
| Never policy: direct idle | 421.07 | 228.76 | 0.47 |
| Never policy: automated local playback | 482.86 | 284.27 | 2.45 |
| Never + prelaunch switch: direct idle | 405.20 | 215.79 | 0.07 |
| Never + prelaunch switch: automated local playback | 468.77 | 271.74 | 2.25 |

Direct idle was measured for 15 seconds, automated idle for 10 seconds, and local
playback for 15 seconds. The same functional checks passed in each configuration.
The effective WebKit setting was logged as Never. The prelaunch switch is
`WEBKIT_DISABLE_COMPOSITING_MODE=1`; the default app behavior is unchanged.
Screenshots confirm the local UI still renders. No hardware device or network
playback check follows from these results. CPU passes this short unthrottled
comparison; memory remains above budget under both metrics. No performance pass
or completed architecture is declared.


## Native account diagnostic iteration

Short virtual Linux measurements after adding the account screen, with WebKit
Never policy and prelaunch compositing switch (not default/physical GPU behavior):

| State | Median summed RSS MiB | Median PSS MiB | CPU % of one core |
| --- | ---: | ---: | ---: |
| Visible idle, automated | 429.11 | 235.78 | 0.42 |
| Local WAV playback, automated | 473.07 | 277.40 | 1.91 |

Evidence: `reports/account-desktop.json`. The entire app/helper tree is measured;
external test infrastructure is excluded. Memory still fails the original target.
This does not measure live Google consent, real YouTube playback, downloads,
Windows/macOS behavior, or the two-hour soak/throttle requirements.


## Streamlined welcome and native network implementation

Short virtual Linux / WebDriver run using Never graphics policy plus the
prelaunch compositing switch:

| State | Median summed RSS MiB | Median PSS MiB | CPU % of one core |
| --- | ---: | ---: | ---: |
| Visible streamlined welcome | 431.46 | 234.80 | 0.10 |
| Separate diagnostics, local WAV playback | 476.50 | 278.59 | 1.16 |

Evidence: `reports/streamlined-desktop.json`. Memory still fails. These are not
real YouTube Music or Windows/macOS numbers. Native loopback HTTP playback passed
functional tests, but no Music performance, physical audio, soak, or throttle
claim follows from it. The portable audio runtime is not yet bundled.
