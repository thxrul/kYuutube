# kYuutube

Cross-platform YouTube Music desktop client proposal, currently at **Phase 0**.
A local-audio Tauri/Rust diagnostic exists. There is no working YouTube Music client or production release yet.

Read [PHASE0_REPORT.md](PHASE0_REPORT.md) for the authentication/playback research,
the actual probe results, the proposed architecture, and blockers. The user approved the native feasibility continuation. The current Linux
virtual-display shell fails the memory budget; full implementation and release
await a budget decision and a working authenticated YouTube playback spike. The user can provide Windows testing; Linux/macOS runtime validation must
still be arranged and cannot be inferred from a successful build.

YouTube Music has no official public API. Any unofficial integration may break.
No downloading, offline audio saving, ad blocking/skipping, or Premium bypass is
implemented or planned. Ads and account entitlements must be preserved by any
future playback integration; free-account playback is currently unverified.
No telemetry or credential collection is implemented. The desktop diagnostic
loads only its local UI and a configured local audio sample. The research tools make
live requests only to YouTube/Google endpoints; package installation uses PyPI.

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

No Google client is needed for these anonymous checks. Do not put credentials in
the checkout or chat. Google Cloud setup and secure first-run authorization
instructions will depend on the architecture decision recorded in the report.

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
