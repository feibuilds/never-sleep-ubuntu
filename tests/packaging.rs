#[test]
fn installer_starts_the_menu_bar_immediately() {
    let script = include_str!("../scripts/install.sh");
    assert!(
        script.contains("systemd-run --user --unit=never-sleep-tray"),
        "install must start the tray in the current GNOME session"
    );
}

#[test]
fn tray_has_single_instance_protection() {
    let script = include_str!("../scripts/tray.py");
    assert!(script.contains("fcntl.LOCK_EX | fcntl.LOCK_NB"));
    assert!(script.contains("never-sleep-tray.lock"));
}

#[test]
fn desktop_entry_uses_the_never_sleep_product_name() {
    let desktop = include_str!("../packaging/never-sleep.desktop");
    assert!(desktop.contains("Name=Never Sleep for Ubuntu"));
    assert!(desktop.contains("Exec=never-sleep-tray"));
}

#[test]
fn cargo_metadata_points_at_the_renamed_github_repo() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("repository = \"https://github.com/feibuilds/never-sleep-ubuntu\""));
    assert!(manifest.contains("homepage = \"https://github.com/feibuilds/never-sleep-ubuntu\""));
    assert!(!manifest.contains("sleepless-ubuntu"));
}

#[test]
fn installer_replaces_the_old_sleepless_ubuntu_names() {
    let script = include_str!("../scripts/install.sh");
    assert!(script.contains("sleepless-ubuntu-tray.service"));
    assert!(script.contains("$bin_dir/sleepless-ubuntu"));
    assert!(script.contains("$autostart_dir/sleepless-ubuntu.desktop"));
}
