# Never Sleep for Ubuntu

[English](README.md) | [简体中文](README.zh-CN.md)

[![CI](https://github.com/pandadolphin/never-sleep-ubuntu/actions/workflows/ci.yml/badge.svg)](https://github.com/pandadolphin/never-sleep-ubuntu/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

让 Ubuntu laptop 在 **display 关闭时仍保持 awake**。这是参考 [mac-never-sleep](https://github.com/xyz-ai-app/mac-never-sleep) 开发的 Ubuntu/GNOME X11 版本，并已在 Ubuntu 22.04 上测试。

它只做一件事：daemon 存活期间持有 process-scoped `systemd-logind` inhibitor，同时让 display 进入真正的 DPMS off。它不会修改 GNOME power settings、`/etc/systemd/logind.conf` 或其他持久电源策略。

## 行为

- `on` 在 `systemd-inhibit` 下启动后台 daemon，然后立即返回。
- 启动约 1.5 秒后，通过 `xset dpms force off` 关闭 display。
- 键盘或鼠标可正常唤醒 display。本地用户操作期间 daemon 不会抢走屏幕；GNOME idle 达到 45 秒后才会再次关屏。
- 仅在 daemon 存活期间 inhibit `idle`、`sleep` 和 `handle-lid-switch`；退出或 crash 会由系统自动释放。
- 使用 battery 时，电量低于 20% 会结束 session；指定 duration 到期也会结束。
- GNOME tray 和 CLI 控制同一个 daemon state。

合盖运行仍取决于硬件和 firmware。无人值守时，lid open + panel asleep 是更可靠、也更利于散热的方式。

## 构建与使用

依赖：Ubuntu GNOME on X11、Rust 1.88、`systemd-inhibit`、`xset`、`gdbus` 和 `notify-send`。

在 Ubuntu 安装 desktop dependencies：

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

Duration 支持 `indefinite`、小时（`8h`）、分钟（`30m`）和本地时间点（`until=08:00`）。

为当前用户安装 CLI 和 tray：

```bash
./scripts/install.sh
```

安装脚本会向 `~/.local/bin` 放入两个 executable，向 `~/.config/autostart` 放入一个 GNOME autostart entry，并立即启动 menu bar indicator。Runtime lock 会避免重复 icon；安装过程不使用 `sudo`。卸载：

```bash
./scripts/uninstall.sh
```

## Agent API

Command 和 JSON vocabulary 与参考项目保持对应：

```bash
never-sleep on --for 8h
never-sleep status --json
never-sleep off
```

顶层 `status` object 中的稳定字段为 `active`、`display`、`lid`、`on_ac`、`battery`、`remaining_secs`、`user_present`、`elapsed_secs`、`stop_reason`、`stop_reason_code`、`screen_off_enabled` 和 `lid_awake_enabled`。

Runtime state 默认位于 `$XDG_STATE_HOME/never-sleep`；未设置时使用 `~/.local/state/never-sleep`。测试可用 `NEVER_SLEEP_DATA_DIR` override。

## 工作原理

| Capability | Mechanism | Persistence |
| --- | --- | --- |
| 阻止 idle/system sleep | `systemd-inhibit --what=idle:sleep:handle-lid-switch --mode=block` | 仅 process lifetime |
| 关闭 display | `xset dpms force off` | 单次请求，不改设置 |
| 检测本地用户 | 通过 session D-Bus 调用 GNOME Mutter `IdleMonitor.GetIdletime` | read-only |
| battery 保护 | `/sys/class/power_supply` | read-only |
| Tray | GTK 3 + Ayatana AppIndicator | 用户 autostart entry |

当前实现明确针对 GNOME X11 session。Wayland 需要不同的 display-off backend；`doctor` 会报告当前 session type，而不会假装 `xset` 在 Wayland 有效。

## 开发

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
```

欢迎贡献。请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)、[SECURITY.md](SECURITY.md) 和 [Code of Conduct](CODE_OF_CONDUCT.md)。

## License

MIT
