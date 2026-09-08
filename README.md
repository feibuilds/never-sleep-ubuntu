# Never Sleep for Ubuntu

[English](README.md) | [简体中文](README.zh-CN.md)

[![CI](https://github.com/feibuilds/never-sleep-ubuntu/actions/workflows/ci.yml/badge.svg)](https://github.com/feibuilds/never-sleep-ubuntu/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Keep an Ubuntu laptop **awake with the display off**. This is an Ubuntu/GNOME X11 counterpart to [mac-never-sleep](https://github.com/xyz-ai-app/mac-never-sleep), developed and tested on Ubuntu 22.04.

It has one narrow job: hold a process-scoped `systemd-logind` inhibitor while using real DPMS display sleep. It does not change GNOME power settings, `/etc/systemd/logind.conf`, or any other persistent power policy.

## Behaviour

- `on` starts a detached daemon under `systemd-inhibit` and returns immediately.
- The display turns off about 1.5 seconds after start via `xset dpms force off`.
- Keyboard or mouse activity wakes the display normally. The daemon does not turn it off again while the local user is active; it waits for 45 seconds of GNOME idle time.
- `idle`, `sleep`, and `handle-lid-switch` are inhibited only while the daemon lives. Quit or crash releases them automatically.
- On battery, the session ends below 20%. A requested duration also ends the session.
- The GNOME tray and CLI control the same daemon state.

Closed-lid operation is still hardware- and firmware-dependent. Keep the lid open with the panel asleep for the most reliable and thermally safe unattended operation.

## Build and use

Requirements: Ubuntu GNOME on X11, Rust 1.88, `systemd-inhibit`, `xset`, `gdbus`, and `notify-send`.

Install the desktop dependencies on Ubuntu:

```bash
sudo apt install x11-utils libnotify-bin python3-gi gir1.2-ayatanaappindicator3-0.1 zenity
```

```bash
cargo build --release
./target/release/never-sleep doctor
./target/release/never-sleep on --for 8h
./target/release/never-sleep status --json
./target/release/never-sleep off
```

Durations are `indefinite`, hours (`8h`), minutes (`30m`), or local wall clock (`until=08:00`).

Install the CLI and tray for the current user:

```bash
./scripts/install.sh
```

The installer places two executables in `~/.local/bin`, adds one GNOME autostart entry in `~/.config/autostart`, and starts the menu-bar indicator immediately. A runtime lock prevents duplicate indicators. It does not use `sudo`. To remove them:

```bash
./scripts/uninstall.sh
```

## Agent API

The command and JSON vocabulary follows the reference project:

```bash
never-sleep on --for 8h
never-sleep status --json
never-sleep off
```

Stable status fields are `active`, `display`, `lid`, `on_ac`, `battery`, `remaining_secs`, `user_present`, `elapsed_secs`, `stop_reason`, `stop_reason_code`, `screen_off_enabled`, and `lid_awake_enabled` inside the top-level `status` object.

Runtime state is stored under `$XDG_STATE_HOME/never-sleep`, or `~/.local/state/never-sleep` when `XDG_STATE_HOME` is unset. `NEVER_SLEEP_DATA_DIR` overrides it for testing.

## How it works

| Capability | Mechanism | Persistence |
| --- | --- | --- |
| Block idle/system sleep | `systemd-inhibit --what=idle:sleep:handle-lid-switch --mode=block` | Process lifetime only |
| Turn the display off | `xset dpms force off` | One request; no setting rewrite |
| Detect local presence | GNOME Mutter `IdleMonitor.GetIdletime` over the session D-Bus | Read-only |
| Battery protection | `/sys/class/power_supply` | Read-only |
| Tray | GTK 3 + Ayatana AppIndicator | User autostart entry |

The implementation currently targets GNOME on X11. Wayland needs a different display-off backend; `doctor` reports the active session type instead of pretending that `xset` works there.

## Development

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md), [SECURITY.md](SECURITY.md), and the [Code of Conduct](CODE_OF_CONDUCT.md).

## License

MIT
