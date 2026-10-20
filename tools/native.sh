#!/usr/bin/env bash
set -euo pipefail
cd /workspace/kYuutube
mkdir -p /workspace/kyuutube-native-cargo
# Keep registry credentials and proxy defaults; explicitly select the managed daemon.
exec env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH \
  docker --host=unix:///var/run/docker.sock run --rm --init --user "$(id -u):$(id -g)" \
  --mount type=bind,src=/workspace/kYuutube,dst=/workspace/kYuutube \
  --mount type=bind,src=/workspace/kyuutube-native-cargo,dst=/workspace/kyuutube-native-cargo \
  --mount type=bind,src=/etc/ssl/certs/ca-certificates.crt,dst=/run/system-ca.pem,readonly \
  -e HOME=/tmp -e CARGO_HTTP_CAINFO=/run/system-ca.pem -e SSL_CERT_FILE=/run/system-ca.pem \
  kyuutube-spike:phase0 "$@"
