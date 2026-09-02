#!/usr/bin/env bash
set -euo pipefail

bin_dir=${XDG_BIN_HOME:-"${HOME}/.local/bin"}
autostart_dir=${XDG_CONFIG_HOME:-"${HOME}/.config"}/autostart

systemctl --user stop never-sleep-tray.service 2>/dev/null || true
systemctl --user stop sleepless-ubuntu-tray.service 2>/dev/null || true
if [[ -x "$bin_dir/never-sleep" ]]; then
    "$bin_dir/never-sleep" off || true
elif [[ -x "$bin_dir/sleepless-ubuntu" ]]; then
    "$bin_dir/sleepless-ubuntu" off || true
fi
rm -f -- \
    "$bin_dir/never-sleep" \
    "$bin_dir/never-sleep-tray" \
    "$autostart_dir/never-sleep.desktop" \
    "$bin_dir/sleepless-ubuntu" \
    "$bin_dir/sleepless-ubuntu-tray" \
    "$autostart_dir/sleepless-ubuntu.desktop"
echo "Never Sleep for Ubuntu was removed. User power settings were not changed."
