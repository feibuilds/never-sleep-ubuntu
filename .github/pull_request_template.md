## Behaviour

Describe the user-visible change and why it belongs in Never Sleep for Ubuntu.

## Evidence

- [ ] Added or updated tests for behaviour changes
- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Documented the Ubuntu version and session type actually tested

## Safety

- [ ] Does not write persistent GNOME or `logind` power settings
- [ ] Does not claim untested Wayland or closed-lid support
