#!/usr/bin/env bash
set -euo pipefail

project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
bin_dir=${XDG_BIN_HOME:-"${HOME}/.local/bin"}
autostart_dir=${XDG_CONFIG_HOME:-"${HOME}/.config"}/autostart

cd "$project_dir"
cargo build --release
install -d "$bin_dir" "$autostart_dir"
install -m 0755 target/release/sleepless-ubuntu "$bin_dir/sleepless-ubuntu"
install -m 0755 scripts/tray.py "$bin_dir/sleepless-ubuntu-tray"
install -m 0644 packaging/sleepless-ubuntu.desktop "$autostart_dir/sleepless-ubuntu.desktop"

if command -v systemd-run >/dev/null 2>&1; then
    systemctl --user stop sleepless-ubuntu-tray.service 2>/dev/null || true
    systemctl --user reset-failed sleepless-ubuntu-tray.service 2>/dev/null || true
    systemd-run --user --unit=sleepless-ubuntu-tray --collect \
        "$bin_dir/sleepless-ubuntu-tray"
else
    setsid "$bin_dir/sleepless-ubuntu-tray" >/dev/null 2>&1 &
fi

echo "Installed CLI and started the GNOME menu-bar indicator."
echo "Run: $bin_dir/sleepless-ubuntu on --for 8h"
