#!/usr/bin/env bash
set -euo pipefail

project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
bin_dir=${XDG_BIN_HOME:-"${HOME}/.local/bin"}
autostart_dir=${XDG_CONFIG_HOME:-"${HOME}/.config"}/autostart

cd "$project_dir"
cargo build --release
install -d "$bin_dir" "$autostart_dir"

if [[ -x "$bin_dir/sleepless-ubuntu" ]]; then
    "$bin_dir/sleepless-ubuntu" off || true
fi
systemctl --user stop sleepless-ubuntu-tray.service 2>/dev/null || true
rm -f -- \
    "$bin_dir/sleepless-ubuntu" \
    "$bin_dir/sleepless-ubuntu-tray" \
    "$autostart_dir/sleepless-ubuntu.desktop"

install -m 0755 target/release/never-sleep "$bin_dir/never-sleep"
install -m 0755 scripts/tray.py "$bin_dir/never-sleep-tray"
install -m 0644 packaging/never-sleep.desktop "$autostart_dir/never-sleep.desktop"

if command -v systemd-run >/dev/null 2>&1; then
    systemctl --user stop never-sleep-tray.service 2>/dev/null || true
    systemctl --user reset-failed never-sleep-tray.service 2>/dev/null || true
    systemd-run --user --unit=never-sleep-tray --collect \
        "$bin_dir/never-sleep-tray"
else
    setsid "$bin_dir/never-sleep-tray" >/dev/null 2>&1 &
fi

echo "Installed CLI and started the GNOME menu-bar indicator."
echo "Run: $bin_dir/never-sleep on --for 8h"
