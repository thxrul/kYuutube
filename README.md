# kYuutube

Cross-platform YouTube Music desktop client proposal, currently at **Phase 0**.
A local-audio Tauri/Rust diagnostic exists. There is no working YouTube Music client or production release yet.

The new [alpha 2 diagnostic](https://github.com/thxrul/kYuutube/releases/tag/v0.0.1-alpha.2)
includes a **system-browser Google OAuth account diagnostic** with OS-keystore storage and read-only YouTube channel/playlist
checks. Live account consent has not been tested. This is not yet a verified
YouTube Music session, streaming client, or Metrolist desktop port.

Read [native account setup](docs/GOOGLE_ACCOUNT.md) and the
[Metrolist feature inventory](docs/METROLIST_PARITY.md). The user's updated scope
includes offline downloads and excludes Last.fm/Discord; downloads are not yet
implemented. No ad skipping, ad blocking, or Premium bypass is implemented.
YouTube Music has no official public API; unofficial integrations may break.
There is no telemetry and the current account implementation talks only to Google/YouTube.

[PHASE0_REPORT.md](PHASE0_REPORT.md) records actual measurements and blockers.
The virtual Linux Tauri shell still fails the original memory budget. YouTube
Music access is still denied by the cloud proxy, and Windows/macOS application
runtime behavior needs validation. A successful compile, OAuth protocol test,
or YouTube Data API query does not establish Music playback or feature parity.

## Reproduce the anonymous feasibility checks

From the existing isolated checkout; do not create a separate worktree:

```sh
cd /workspace/kYuutube
python3 -m venv /workspace/kyuutube-spike-venv
/workspace/kyuutube-spike-venv/bin/python -m pip install --no-input --disable-pip-version-check -r tools/requirements-spike.txt
/workspace/kyuutube-spike-venv/bin/python tools/phase0_probe.py --output reports/local-probes.json
```

Both probes must succeed for exit status 0. A failure is not suppressed. A success
still validates only anonymous search and metadata resolution, **not** login,
audio playback, entitlement handling, a desktop UI, or the performance budget.
Results contain no access tokens, cookies, stream URLs, or provider response
bodies. The tool never imports browser cookies or downloads media. Preserve the
host's proxy and CA settings; do not disable TLS checks to run it.

No Google client is needed for these anonymous checks. For the separate desktop account diagnostic, follow [the Google Cloud setup guide](docs/GOOGLE_ACCOUNT.md). Do not put credentials in the checkout or chat.

## Native diagnostic

Build and run the Linux spike in the prepared development image as documented in
[docs/NATIVE_SPIKE.md](docs/NATIVE_SPIKE.md). The GitHub workflow compiles diagnostic
executables for Windows, Linux, and both macOS architectures; its status must be
checked before claiming those builds succeeded. It does not publish a production
release or certify runtime behavior on Windows/macOS.

## Delivery status

- [Test status](TEST_REPORT.md)
- [Performance status](PERFORMANCE.md)
- [Design audit status](MD3_AUDIT.md)

Windows, macOS, and Linux production installers will follow a successful
spike and architecture sign-off. No production installer or release is claimed now.

## Current diagnostic downloads

[Alpha 2 release](https://github.com/thxrul/kYuutube/releases/tag/v0.0.1-alpha.2)
contains Windows x64, Linux x64, macOS Intel, and macOS Apple Silicon executables,
setup guides, a sample WAV, a measurement helper, and SHA-256 checksums.
[Download the Windows ZIP](https://github.com/thxrul/kYuutube/releases/download/v0.0.1-alpha.2/kYuutube-v0.0.1-alpha.2-windows-x86_64.zip),
extract it, and launch `kyuutube-desktop-spike.exe`. Follow the included
`GOOGLE_ACCOUNT.md` for your own Desktop OAuth client and browser consent.

[Build run 39567857094](https://github.com/thxrul/kYuutube/actions/runs/39567857094)
compiled all four targets. Thirteen account tests passed on each of Windows,
Linux, and Apple Silicon macOS; Intel macOS was cross-compiled without executing
them. Publication downloaded the public assets and verified their checksums.
See [the build verification record](reports/account-ci-builds.json).

These are unsigned diagnostics, not installers. Real account consent and OS
credential persistence still need testing. YouTube Music streaming, offline
downloads, and full Metrolist parity are not included yet. The local audio test
requires a separate trusted libmpv runtime; account tests do not require it.
