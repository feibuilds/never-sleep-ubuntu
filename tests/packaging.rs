#[test]
fn installer_starts_the_menu_bar_immediately() {
    let script = include_str!("../scripts/install.sh");
    assert!(
        script.contains("systemd-run --user --unit=sleepless-ubuntu-tray"),
        "install must start the tray in the current GNOME session"
    );
}

#[test]
fn tray_has_single_instance_protection() {
    let script = include_str!("../scripts/tray.py");
    assert!(script.contains("fcntl.LOCK_EX | fcntl.LOCK_NB"));
    assert!(script.contains("sleepless-ubuntu-tray.lock"));
}
