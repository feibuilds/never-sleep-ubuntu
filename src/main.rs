use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use chrono::{Local, LocalResult, TimeZone};
use clap::{Parser, Subcommand};
use never_sleep_ubuntu::{
    command_exists, data_dir, inhibitor_args, parse_duration, parse_gnome_idle_ms,
    read_power_supply, should_sleep_display, stop_signal_args, DisplayPolicy, DurationSpec,
};
use serde::{Deserialize, Serialize};

const DISPLAY_OFF_DELAY_MS: u64 = 1_500;
const USER_IDLE_RESLEEP_MS: u64 = 45_000;
const BATTERY_FLOOR: u8 = 20;

#[derive(Debug, Parser)]
#[command(
    name = "never-sleep",
    version,
    about = "Keep an Ubuntu laptop awake with the display off"
)]
struct Cli {
    #[command(subcommand)]
    command: CommandKind,
}

#[derive(Debug, Subcommand)]
enum CommandKind {
    /// Start a background screen-off session.
    On {
        /// Duration: indefinite, 3h, 30m, or until=08:00.
        #[arg(long, default_value = "indefinite")]
        r#for: String,
        #[arg(long)]
        json: bool,
    },
    /// End the current session.
    Off {
        #[arg(long)]
        json: bool,
    },
    /// Start or end the current session.
    Toggle {
        #[arg(long)]
        json: bool,
    },
    /// Show current host and session state.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Diagnose the Ubuntu power and display integration.
    Doctor,
    /// Internal process held by systemd-inhibit.
    #[command(hide = true)]
    Daemon {
        #[arg(long, default_value = "indefinite")]
        r#for: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredSession {
    pid: u32,
    started_unix: i64,
    deadline_unix: Option<i64>,
    duration: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LastStop {
    code: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct JsonStatus {
    active: bool,
    display: String,
    lid: String,
    on_ac: bool,
    battery: Option<u8>,
    remaining_secs: Option<u64>,
    user_present: bool,
    elapsed_secs: Option<u64>,
    stop_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_reason_code: Option<String>,
    screen_off_enabled: bool,
    lid_awake_enabled: bool,
}

#[derive(Debug, Serialize)]
struct Response {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    status: JsonStatus,
}

fn main() {
    let result = match Cli::parse().command {
        CommandKind::On { r#for, json } => start(&r#for, json),
        CommandKind::Off { json } => stop(json),
        CommandKind::Toggle { json } => {
            if live_session().is_some() {
                stop(json)
            } else {
                start("indefinite", json)
            }
        }
        CommandKind::Status { json } => {
            print_status(json);
            Ok(())
        }
        CommandKind::Doctor => {
            doctor();
            Ok(())
        }
        CommandKind::Daemon { r#for } => run_daemon(&r#for),
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn session_path() -> PathBuf {
    data_dir().join("session.json")
}

fn last_stop_path() -> PathBuf {
    data_dir().join("last-stop.json")
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("tmp");
    fs::write(
        &tmp,
        serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::rename(tmp, path).map_err(|e| e.to_string())
}

fn process_alive(pid: u32) -> bool {
    PathBuf::from(format!("/proc/{pid}")).exists()
}

fn live_session() -> Option<StoredSession> {
    let session: StoredSession = read_json(&session_path())?;
    if process_alive(session.pid) {
        Some(session)
    } else {
        let _ = fs::remove_file(session_path());
        None
    }
}

fn start(duration: &str, json: bool) -> Result<(), String> {
    parse_duration(duration)?;
    if live_session().is_none() {
        if !command_exists("systemd-inhibit") {
            return Err("systemd-inhibit is required".into());
        }
        fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(data_dir().join("daemon.log"))
            .map_err(|e| e.to_string())?;
        let err = log.try_clone().map_err(|e| e.to_string())?;
        let mut child = Command::new("systemd-inhibit")
            .args(inhibitor_args(&exe, duration))
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(err))
            .spawn()
            .map_err(|e| format!("failed to start systemd-inhibit: {e}"))?;
        for _ in 0..30 {
            if live_session().is_some() {
                break;
            }
            if child.try_wait().map_err(|e| e.to_string())?.is_some() {
                return Err(format!(
                    "session failed to start; see {}",
                    data_dir().join("daemon.log").display()
                ));
            }
            thread::sleep(Duration::from_millis(100));
        }
        if live_session().is_none() {
            return Err("session did not become ready within 3 seconds".into());
        }
    }
    if json {
        print_json_response(true, None);
    } else {
        println!("Never Sleep for Ubuntu is on. The display will turn off in about 1.5 seconds.");
    }
    Ok(())
}

fn stop(json: bool) -> Result<(), String> {
    if let Some(session) = live_session() {
        let status = Command::new("kill")
            .args(stop_signal_args(session.pid))
            .status()
            .map_err(|e| format!("failed to stop daemon: {e}"))?;
        if !status.success() {
            return Err("daemon did not accept SIGINT".into());
        }
        for _ in 0..30 {
            if live_session().is_none() {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
    if json {
        print_json_response(true, None);
    } else {
        println!("Never Sleep for Ubuntu is off. Normal Ubuntu power policy is in control.");
    }
    Ok(())
}

fn deadline_for(spec: DurationSpec, now: chrono::DateTime<Local>) -> Option<i64> {
    match spec {
        DurationSpec::Indefinite => None,
        DurationSpec::Seconds(seconds) => Some(now.timestamp().saturating_add(seconds as i64)),
        DurationSpec::Until { hour, minute } => {
            let date = now.date_naive();
            let naive = date.and_hms_opt(hour.into(), minute.into(), 0)?;
            let mut target = match Local.from_local_datetime(&naive) {
                LocalResult::Single(value) | LocalResult::Ambiguous(value, _) => value,
                LocalResult::None => return None,
            };
            if target <= now {
                let tomorrow = date.succ_opt()?;
                let naive = tomorrow.and_hms_opt(hour.into(), minute.into(), 0)?;
                target = Local.from_local_datetime(&naive).earliest()?;
            }
            Some(target.timestamp())
        }
    }
}

fn acquire_session_file(session: &StoredSession) -> Result<File, String> {
    fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
    if let Some(existing) = live_session() {
        return Err(format!("session already running as pid {}", existing.pid));
    }
    let path = session_path();
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .or_else(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                fs::remove_file(&path)?;
                OpenOptions::new().write(true).create_new(true).open(&path)
            } else {
                Err(error)
            }
        })
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&file, session).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    Ok(file)
}

fn run_daemon(duration: &str) -> Result<(), String> {
    let spec = parse_duration(duration)?;
    let now = Local::now();
    let session = StoredSession {
        pid: std::process::id(),
        started_unix: now.timestamp(),
        deadline_unix: deadline_for(spec, now),
        duration: duration.into(),
    };
    let _guard = acquire_session_file(&session)?;
    let running = Arc::new(AtomicBool::new(true));
    let signal_running = Arc::clone(&running);
    ctrlc::set_handler(move || signal_running.store(false, Ordering::SeqCst))
        .map_err(|e| e.to_string())?;

    notify(
        "Screen-off session started",
        "Ubuntu will stay awake while the display is off.",
    );
    let start = Instant::now();
    let mut initial_sleep_sent = false;
    let mut last_sleep_ms = None;
    let mut stop_reason = LastStop {
        code: "user".into(),
        message: "Ended by you".into(),
    };

    while running.load(Ordering::SeqCst) {
        let now_unix = Local::now().timestamp();
        if session
            .deadline_unix
            .is_some_and(|deadline| now_unix >= deadline)
        {
            stop_reason = LastStop {
                code: "duration_elapsed".into(),
                message: "The selected duration elapsed".into(),
            };
            break;
        }
        let (on_ac, battery) = read_power_supply(Path::new("/sys/class/power_supply"));
        if !on_ac && battery.is_some_and(|percent| percent < BATTERY_FLOOR) {
            stop_reason = LastStop {
                code: "battery_floor".into(),
                message: format!("Battery fell below {BATTERY_FLOOR}%"),
            };
            break;
        }

        let now_ms = start.elapsed().as_millis() as u64;
        let idle_ms = gnome_idle_ms().unwrap_or(0);
        let policy = DisplayPolicy {
            started_ms: 0,
            initial_sleep_sent,
            last_sleep_ms,
            display_off_delay_ms: DISPLAY_OFF_DELAY_MS,
            idle_resleep_ms: USER_IDLE_RESLEEP_MS,
        };
        if should_sleep_display(policy, now_ms, idle_ms) {
            if let Err(error) = sleep_display() {
                eprintln!("display sleep failed: {error}");
            } else {
                initial_sleep_sent = true;
                last_sleep_ms = Some(now_ms);
            }
        }
        thread::sleep(if initial_sleep_sent {
            Duration::from_secs(2)
        } else {
            Duration::from_millis(250)
        });
    }

    let _ = write_json(&last_stop_path(), &stop_reason);
    let _ = fs::remove_file(session_path());
    if stop_reason.code != "user" {
        notify("Screen-off session ended", &stop_reason.message);
    }
    Ok(())
}

fn gnome_idle_ms() -> Option<u64> {
    let output = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.gnome.Mutter.IdleMonitor",
            "--object-path",
            "/org/gnome/Mutter/IdleMonitor/Core",
            "--method",
            "org.gnome.Mutter.IdleMonitor.GetIdletime",
        ])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| parse_gnome_idle_ms(&String::from_utf8_lossy(&output.stdout)))
        .flatten()
}

fn sleep_display() -> Result<(), String> {
    if std::env::var_os("NEVER_SLEEP_NO_DISPLAY_OFF").is_some() {
        return Ok(());
    }
    let status = Command::new("xset")
        .args(["dpms", "force", "off"])
        .status()
        .map_err(|e| e.to_string())?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| "xset dpms force off exited unsuccessfully".into())
}

fn display_state() -> String {
    let output = Command::new("xset").arg("q").output();
    match output {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout);
            if text.contains("Monitor is Off") {
                "asleep".into()
            } else if text.contains("Monitor is On") {
                "awake".into()
            } else {
                "unknown".into()
            }
        }
        _ => "unknown".into(),
    }
}

fn lid_state() -> String {
    let roots = ["/proc/acpi/button/lid", "/sys/class/power_supply"];
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            for name in ["state", "status"] {
                let Ok(value) = fs::read_to_string(entry.path().join(name)) else {
                    continue;
                };
                let lower = value.to_ascii_lowercase();
                if lower.contains("closed") {
                    return "closed".into();
                }
                if lower.contains("open") {
                    return "open".into();
                }
            }
        }
    }
    "unknown".into()
}

fn current_status() -> JsonStatus {
    let session = live_session();
    let now = Local::now().timestamp();
    let (on_ac, battery) = read_power_supply(Path::new("/sys/class/power_supply"));
    let idle = gnome_idle_ms();
    let last: Option<LastStop> = read_json(&last_stop_path());
    JsonStatus {
        active: session.is_some(),
        display: display_state(),
        lid: lid_state(),
        on_ac,
        battery,
        remaining_secs: session
            .as_ref()
            .and_then(|s| s.deadline_unix)
            .map(|deadline| deadline.saturating_sub(now).max(0) as u64),
        user_present: lid_state() != "closed"
            && idle.is_some_and(|value| value < USER_IDLE_RESLEEP_MS),
        elapsed_secs: session
            .as_ref()
            .map(|s| now.saturating_sub(s.started_unix).max(0) as u64),
        stop_reason: session
            .is_none()
            .then(|| last.as_ref().map(|v| v.message.clone()))
            .flatten(),
        stop_reason_code: session.is_none().then(|| last.map(|v| v.code)).flatten(),
        screen_off_enabled: true,
        lid_awake_enabled: true,
    }
}

fn print_json_response(ok: bool, error: Option<String>) {
    println!(
        "{}",
        serde_json::to_string_pretty(&Response {
            ok,
            error,
            status: current_status(),
        })
        .expect("status must serialize")
    );
}

fn print_status(json: bool) {
    let status = current_status();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&Response {
                ok: true,
                error: None,
                status,
            })
            .expect("status must serialize")
        );
    } else if status.active {
        println!(
            "On · display {} · lid {} · {} · battery {}%",
            status.display,
            status.lid,
            if status.on_ac { "AC" } else { "battery" },
            status.battery.map_or_else(|| "?".into(), |v| v.to_string())
        );
    } else {
        println!("Off · normal Ubuntu power policy is in control");
    }
}

fn doctor() {
    println!("Never Sleep for Ubuntu doctor");
    println!(
        "session: {}",
        if live_session().is_some() {
            "active"
        } else {
            "inactive"
        }
    );
    println!(
        "session type: {}",
        std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".into())
    );
    println!(
        "DISPLAY: {}",
        std::env::var("DISPLAY").unwrap_or_else(|_| "missing".into())
    );
    for command in ["systemd-inhibit", "xset", "gdbus", "notify-send"] {
        println!(
            "{command}: {}",
            if command_exists(command) {
                "ok"
            } else {
                "missing"
            }
        );
    }
    let status = current_status();
    println!("display: {}", status.display);
    println!("lid: {}", status.lid);
    println!("power: {}", if status.on_ac { "AC" } else { "battery" });
    println!(
        "battery: {}",
        status
            .battery
            .map_or_else(|| "unknown".into(), |v| format!("{v}%"))
    );
    println!(
        "policy: process-scoped systemd inhibitor; no persistent GNOME or logind settings changed"
    );
}

fn notify(title: &str, body: &str) {
    if std::env::var_os("NEVER_SLEEP_NO_NOTIFY").is_some() {
        return;
    }
    let _ = Command::new("notify-send")
        .args(["--app-name=Never Sleep for Ubuntu", title, body])
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_status_keeps_agent_field_names() {
        let value = serde_json::to_value(JsonStatus {
            active: true,
            display: "asleep".into(),
            lid: "open".into(),
            on_ac: true,
            battery: Some(80),
            remaining_secs: Some(3_600),
            user_present: false,
            elapsed_secs: Some(10),
            stop_reason: None,
            stop_reason_code: None,
            screen_off_enabled: true,
            lid_awake_enabled: true,
        })
        .unwrap();
        for key in [
            "active",
            "display",
            "lid",
            "on_ac",
            "battery",
            "remaining_secs",
            "user_present",
            "elapsed_secs",
            "stop_reason",
            "screen_off_enabled",
            "lid_awake_enabled",
        ] {
            assert!(value.get(key).is_some(), "missing {key}");
        }
        assert!(value.get("stop_reason_code").is_none());
    }

    #[test]
    fn until_deadline_always_targets_the_future() {
        let now = Local.with_ymd_and_hms(2026, 9, 1, 9, 0, 0).unwrap();
        let deadline = deadline_for(DurationSpec::Until { hour: 8, minute: 0 }, now).unwrap();
        assert!(deadline > now.timestamp());
        assert!(deadline - now.timestamp() <= 24 * 60 * 60);
    }
}
