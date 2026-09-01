#!/usr/bin/env bash
set -euo pipefail

bin_dir=${XDG_BIN_HOME:-"${HOME}/.local/bin"}
autostart_dir=${XDG_CONFIG_HOME:-"${HOME}/.config"}/autostart

systemctl --user stop sleepless-ubuntu-tray.service 2>/dev/null || true
if [[ -x "$bin_dir/sleepless-ubuntu" ]]; then
    "$bin_dir/sleepless-ubuntu" off || true
fi
rm -f -- \
    "$bin_dir/sleepless-ubuntu" \
    "$bin_dir/sleepless-ubuntu-tray" \
    "$autostart_dir/sleepless-ubuntu.desktop"
echo "Sleepless Ubuntu was removed. User power settings were not changed."
