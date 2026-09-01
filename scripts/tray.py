#!/usr/bin/env python3
"""Small GNOME tray controller for the sleepless-ubuntu daemon."""

import fcntl
import os
import shutil
import subprocess

import gi

gi.require_version("Gtk", "3.0")
gi.require_version("AyatanaAppIndicator3", "0.1")
from gi.repository import AyatanaAppIndicator3, GLib, Gtk  # noqa: E402


APP_ID = "sleepless-ubuntu"
BINARY = shutil.which("sleepless-ubuntu") or os.path.expanduser(
    "~/.local/bin/sleepless-ubuntu"
)
LOCK_HANDLE = None


def acquire_instance_lock():
    """Keep one indicator per login session, including repeated installs."""
    global LOCK_HANDLE
    runtime_dir = os.environ.get("XDG_RUNTIME_DIR", f"/run/user/{os.getuid()}")
    lock_path = os.path.join(runtime_dir, "sleepless-ubuntu-tray.lock")
    LOCK_HANDLE = open(lock_path, "w", encoding="utf-8")
    try:
        fcntl.flock(LOCK_HANDLE, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        return False
    LOCK_HANDLE.write(str(os.getpid()))
    LOCK_HANDLE.flush()
    return True


def is_active():
    result = subprocess.run(
        [BINARY, "status", "--json"], capture_output=True, text=True, check=False
    )
    return '"active": true' in result.stdout


class Tray:
    def __init__(self):
        self.indicator = AyatanaAppIndicator3.Indicator.new(
            APP_ID,
            "weather-clear-night-symbolic",
            AyatanaAppIndicator3.IndicatorCategory.APPLICATION_STATUS,
        )
        self.indicator.set_status(AyatanaAppIndicator3.IndicatorStatus.ACTIVE)
        self.menu = Gtk.Menu()
        self.status_item = Gtk.MenuItem(label="Checking status…")
        self.status_item.set_sensitive(False)
        self.toggle_item = Gtk.MenuItem(label="Start Screen-Off Session")
        self.toggle_item.connect("activate", self.toggle)
        doctor_item = Gtk.MenuItem(label="Doctor…")
        doctor_item.connect("activate", self.doctor)
        quit_item = Gtk.MenuItem(label="Quit Tray")
        quit_item.connect("activate", lambda _item: Gtk.main_quit())
        for item in [
            self.status_item,
            Gtk.SeparatorMenuItem(),
            self.toggle_item,
            doctor_item,
            Gtk.SeparatorMenuItem(),
            quit_item,
        ]:
            self.menu.append(item)
        self.menu.show_all()
        self.indicator.set_menu(self.menu)
        self.refresh()
        GLib.timeout_add_seconds(3, self.refresh)

    def refresh(self):
        active = is_active()
        self.status_item.set_label("Screen-off session: On" if active else "Screen-off session: Off")
        self.toggle_item.set_label("End Session" if active else "Start Screen-Off Session")
        self.indicator.set_icon_full(
            "weather-clear-night-symbolic" if active else "weather-clear-symbolic",
            "Sleepless Ubuntu on" if active else "Sleepless Ubuntu off",
        )
        return True

    def toggle(self, _item):
        subprocess.Popen([BINARY, "toggle"], start_new_session=True)
        GLib.timeout_add(500, self.refresh)

    def doctor(self, _item):
        result = subprocess.run(
            [BINARY, "doctor"], capture_output=True, text=True, check=False
        )
        subprocess.Popen(
            ["zenity", "--info", "--title=Sleepless Ubuntu", f"--text={result.stdout}"],
            start_new_session=True,
        )


if __name__ == "__main__":
    if not acquire_instance_lock():
        raise SystemExit(0)
    Tray()
    Gtk.main()
