#!/usr/bin/env bash
set -euo pipefail
cd /workspace/kYuutube
export HOME=/tmp/kyuutube-spike-home
export XDG_RUNTIME_DIR=/tmp/kyuutube-spike-runtime
mkdir -p "$HOME" "$XDG_RUNTIME_DIR"
chmod 700 "$HOME" "$XDG_RUNTIME_DIR"
export KYUUTUBE_SPIKE_SAMPLE=/workspace/kYuutube/tools/fixtures/tone.wav
python3 tools/make_sample.py
pulseaudio --daemonize --exit-idle-time=-1 --log-target=file:/tmp/kyuutube-pulse.log
trap 'pulseaudio --kill || true' EXIT
pactl load-module module-null-sink sink_name=kyuutube_test
pactl set-default-sink kyuutube_test
if [ "${1:-}" = "direct" ]; then
  dbus-run-session -- xvfb-run -a -s '-screen 0 1920x1080x24' python3 tools/direct_window_probe.py
  exit
fi
if [ "${1:-}" != "desktop-only" ]; then
  python3 tools/audio_probe.py
fi
dbus-run-session -- xvfb-run -a -s '-screen 0 1920x1080x24' python3 tools/desktop_probe.py
