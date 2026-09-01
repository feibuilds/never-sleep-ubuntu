# Contributing

Thanks for helping improve Sleepless Ubuntu.

## Scope

The product goal is deliberately narrow: keep an Ubuntu laptop awake with the display off. Changes must preserve these rules:

1. Display sleep and system sleep prevention remain separate.
2. Local keyboard or mouse activity retains control of the display.
3. Power inhibition is process-scoped. Do not rewrite GNOME settings or `logind.conf`.
4. `sleepless-ubuntu on --for 8h` and existing JSON field names remain stable.
5. Closed-lid operation must be described as hardware- and firmware-dependent.

## Development setup

The supported development baseline is Ubuntu GNOME on X11 with Rust 1.88.

```bash
sudo apt install x11-utils libnotify-bin python3-gi gir1.2-ayatanaappindicator3-0.1 zenity desktop-file-utils shellcheck
cargo test
```

Use test-driven development for behaviour changes: add a failing test, make the smallest implementation change, then refactor with tests green.

Before opening a pull request, run:

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
python3 -m py_compile scripts/tray.py
shellcheck scripts/*.sh
desktop-file-validate packaging/sleepless-ubuntu.desktop
```

Keep pull requests focused. Explain the user-visible behaviour, the test that locks it, and any Ubuntu/session types you actually tested. Do not claim Wayland or closed-lid behaviour without real hardware evidence.
