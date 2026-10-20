#!/usr/bin/env bash
set -euo pipefail
cd /workspace/kYuutube
# Use the existing checkout. Cloud tasks are already isolated; no worktree needed.
env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH \
  docker --host=unix:///var/run/docker.sock info >/dev/null
if ! env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH \
  docker --host=unix:///var/run/docker.sock image inspect kyuutube-spike:phase0 >/dev/null 2>&1; then
  BUILDX_CONFIG=/workspace/kyuutube-buildx \
    env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH \
    docker --host=unix:///var/run/docker.sock build \
      --secret id=system_ca,src=/etc/ssl/certs/ca-certificates.crt \
      -f tools/Dockerfile.spike -t kyuutube-spike:phase0 .
fi
python3 -m venv /workspace/kyuutube-spike-venv
/workspace/kyuutube-spike-venv/bin/python -m pip install --no-input --disable-pip-version-check --no-cache-dir -r tools/requirements-spike.txt
/workspace/kyuutube-spike-venv/bin/python -m pip check
python3 tools/make_sample.py
bash tools/native.sh cargo build --release --locked --workspace
bash tools/native.sh cargo test --locked -p kyuutube-audio-spike
