# Changelog

All notable changes to this project will be documented in this file. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and releases use [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Renamed the product from Sleepless Ubuntu to Never Sleep for Ubuntu.
- CLI, tray, autostart, and state directory now use `never-sleep` names. `install.sh` and `uninstall.sh` still remove the old `sleepless-ubuntu` files.
- GitHub repository is now `pandadolphin/never-sleep-ubuntu`.

### Added

- Process-scoped `systemd-inhibit` daemon for keeping Ubuntu awake.
- Real DPMS display sleep with local-user idle protection.
- Battery floor and duration-based automatic stop conditions.
- Agent-friendly CLI and stable JSON status fields.
- GNOME AppIndicator menu with current-session launch and login autostart.
- English and Simplified Chinese documentation.

[Unreleased]: https://github.com/pandadolphin/never-sleep-ubuntu/commits/main
