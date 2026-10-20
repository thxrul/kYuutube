# Native feasibility workspace

Use the existing isolated `/workspace/kYuutube` checkout. Do not create a Git
worktree unless requested. The prepared `kyuutube-spike:phase0` Docker image
contains Rust 1.92.0, native Tauri dependencies, libmpv, Xvfb, virtual PulseAudio,
and WebDriver. Live processes are not assumed to survive publication or a new task.

If the image is missing, follow the cloud runtime's Docker/proxy/CA guidance and
rebuild without changing registry credentials or disabling TLS verification:

```sh
cd /workspace/kYuutube
BUILDX_CONFIG=/workspace/kyuutube-buildx \
  env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH \
  docker --host=unix:///var/run/docker.sock build \
    --secret id=system_ca,src=/etc/ssl/certs/ca-certificates.crt \
    -f tools/Dockerfile.spike -t kyuutube-spike:phase0 .
```

Build and check the spike:

```sh
python3 tools/make_sample.py
bash tools/native.sh cargo build --release --locked --workspace
bash tools/native.sh cargo test --locked -p kyuutube-audio-spike
bash tools/native.sh cargo clippy --locked -p kyuutube-audio-spike --all-targets -- -D warnings
```

Run resource checks sequentially, keeping their fixtures and workload consistent:

```sh
bash tools/native.sh bash tools/run_desktop_probe.sh
bash tools/native.sh bash tools/run_desktop_probe.sh direct
```

The first starts/stops its own virtual display/audio services, checks the native
engine, then drives the real Tauri window. `desktop-only` reruns the UI check after
a driver diagnosis; `direct` measures idle without WebDriver. Neither makes a
YouTube request. Results named `reports/local-*` are ignored; reviewed evidence
copies are retained separately. No renderer sandbox override was needed.

A functional exit status 0 does not establish a performance pass. The current
default-renderer memory result fails the target. Read `PERFORMANCE.md` and
`PHASE0_REPORT.md` before choosing further work. Physical sound, Wayland, real
YouTube, soak, and throttled behavior remain unverified.

The GitHub diagnostic workflow compiles raw executables for Windows, Linux, and
both macOS architectures, and runs native tests on Linux. Windows/macOS jobs are
compile checks, not runtime tests. It does not create a production release or
bundle libmpv. See [FEASIBILITY_BUILD.md](FEASIBILITY_BUILD.md) for artifact use.
