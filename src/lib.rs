use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurationSpec {
    Indefinite,
    Seconds(u64),
    Until { hour: u8, minute: u8 },
}

pub fn parse_duration(raw: &str) -> Result<DurationSpec, String> {
    let raw = raw.trim();
    if raw.eq_ignore_ascii_case("indefinite") {
        return Ok(DurationSpec::Indefinite);
    }
    if let Some(clock) = raw.strip_prefix("until=") {
        let (hour, minute) = clock
            .split_once(':')
            .ok_or_else(|| "expected until=HH:MM".to_string())?;
        let hour: u8 = hour.parse().map_err(|_| "invalid hour".to_string())?;
        let minute: u8 = minute.parse().map_err(|_| "invalid minute".to_string())?;
        if hour > 23 || minute > 59 {
            return Err("time must be between 00:00 and 23:59".into());
        }
        return Ok(DurationSpec::Until { hour, minute });
    }
    let (number, factor) = match raw.chars().last() {
        Some('h') => (&raw[..raw.len() - 1], 3_600),
        Some('m') => (&raw[..raw.len() - 1], 60),
        _ => return Err("expected indefinite, Nh, Nm, or until=HH:MM".into()),
    };
    let amount: u64 = number.parse().map_err(|_| "invalid duration".to_string())?;
    if amount == 0 {
        return Err("duration must be greater than zero".into());
    }
    amount
        .checked_mul(factor)
        .map(DurationSpec::Seconds)
        .ok_or_else(|| "duration is too large".into())
}

pub fn parse_gnome_idle_ms(output: &str) -> Option<u64> {
    let marker = "uint64 ";
    let rest = output.split_once(marker)?.1;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

pub fn read_power_supply(root: &Path) -> (bool, Option<u8>) {
    let mut on_ac = false;
    let mut saw_mains = false;
    let mut battery = None;
    let Ok(entries) = fs::read_dir(root) else {
        return (true, None);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let kind = fs::read_to_string(path.join("type")).unwrap_or_default();
        match kind.trim() {
            "Mains" | "USB" | "USB_C" => {
                saw_mains = true;
                if fs::read_to_string(path.join("online")).is_ok_and(|v| v.trim() == "1") {
                    on_ac = true;
                }
            }
            "Battery" => {
                battery = fs::read_to_string(path.join("capacity"))
                    .ok()
                    .and_then(|v| v.trim().parse().ok());
            }
            _ => {}
        }
    }
    (if saw_mains { on_ac } else { true }, battery)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayPolicy {
    pub started_ms: u64,
    pub initial_sleep_sent: bool,
    pub last_sleep_ms: Option<u64>,
    pub display_off_delay_ms: u64,
    pub idle_resleep_ms: u64,
}

pub fn should_sleep_display(policy: DisplayPolicy, now_ms: u64, idle_ms: u64) -> bool {
    if now_ms.saturating_sub(policy.started_ms) < policy.display_off_delay_ms {
        return false;
    }
    if !policy.initial_sleep_sent {
        return true;
    }
    if idle_ms < policy.idle_resleep_ms {
        return false;
    }
    policy
        .last_sleep_ms
        .is_none_or(|last| now_ms.saturating_sub(last) >= 3_000)
}

pub fn inhibitor_args(exe: &Path, duration: &str) -> Vec<String> {
    vec![
        "--what=idle:sleep:handle-lid-switch".into(),
        "--mode=block".into(),
        "--who=Sleepless Ubuntu".into(),
        "--why=Keep remote work running with the display off".into(),
        exe.display().to_string(),
        "daemon".into(),
        "--for".into(),
        duration.into(),
    ]
}

pub fn stop_signal_args(pid: u32) -> [String; 2] {
    ["-INT".into(), pid.to_string()]
}

pub fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .args(["-c", "command -v -- \"$1\" >/dev/null 2>&1", "sh", name])
        .status()
        .is_ok_and(|s| s.success())
}

pub fn data_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("SLEEPLESS_UBUNTU_DATA_DIR") {
        return path.into();
    }
    if let Some(path) = std::env::var_os("XDG_STATE_HOME") {
        return PathBuf::from(path).join("sleepless-ubuntu");
    }
    PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| ".".into()))
        .join(".local/state/sleepless-ubuntu")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn duration_contract_accepts_agent_and_wall_clock_forms() {
        assert_eq!(
            parse_duration("indefinite").unwrap(),
            DurationSpec::Indefinite
        );
        assert_eq!(parse_duration("8h").unwrap(), DurationSpec::Seconds(28_800));
        assert_eq!(parse_duration("30m").unwrap(), DurationSpec::Seconds(1_800));
        assert_eq!(
            parse_duration("until=08:15").unwrap(),
            DurationSpec::Until {
                hour: 8,
                minute: 15
            }
        );
        assert!(parse_duration("0h").is_err());
        assert!(parse_duration("until=25:00").is_err());
    }

    #[test]
    fn mutter_idle_output_is_parsed_without_locale_assumptions() {
        assert_eq!(parse_gnome_idle_ms("(uint64 45123,)\n"), Some(45_123));
        assert_eq!(parse_gnome_idle_ms("Error: no owner"), None);
    }

    #[test]
    fn sysfs_power_supply_reports_ac_and_battery() {
        let dir = tempfile::tempdir().unwrap();
        let ac = dir.path().join("AC0");
        let bat = dir.path().join("BAT0");
        fs::create_dir_all(&ac).unwrap();
        fs::create_dir_all(&bat).unwrap();
        fs::write(ac.join("type"), "Mains\n").unwrap();
        fs::write(ac.join("online"), "0\n").unwrap();
        fs::write(bat.join("type"), "Battery\n").unwrap();
        fs::write(bat.join("capacity"), "37\n").unwrap();
        assert_eq!(read_power_supply(dir.path()), (false, Some(37)));
        fs::write(ac.join("online"), "1\n").unwrap();
        assert_eq!(read_power_supply(dir.path()), (true, Some(37)));
    }

    #[test]
    fn display_policy_sleeps_once_then_waits_for_local_idle() {
        let initial = DisplayPolicy {
            started_ms: 1_000,
            initial_sleep_sent: false,
            last_sleep_ms: None,
            display_off_delay_ms: 1_500,
            idle_resleep_ms: 45_000,
        };
        assert!(!should_sleep_display(initial, 2_499, 0));
        assert!(should_sleep_display(initial, 2_500, 0));

        let resleep = DisplayPolicy {
            initial_sleep_sent: true,
            last_sleep_ms: Some(2_500),
            ..initial
        };
        assert!(!should_sleep_display(resleep, 10_000, 1_000));
        assert!(should_sleep_display(resleep, 50_000, 47_500));
    }

    #[test]
    fn inhibitor_wraps_daemon_without_persistent_power_changes() {
        let args = inhibitor_args(Path::new("/opt/sleepless-ubuntu"), "8h");
        assert_eq!(
            args,
            [
                "--what=idle:sleep:handle-lid-switch",
                "--mode=block",
                "--who=Sleepless Ubuntu",
                "--why=Keep remote work running with the display off",
                "/opt/sleepless-ubuntu",
                "daemon",
                "--for",
                "8h",
            ]
        );
        assert!(!args.iter().any(|arg| arg.contains("pmset")));
    }

    #[test]
    fn stop_uses_the_signal_handled_by_the_daemon() {
        assert_eq!(stop_signal_args(42), ["-INT", "42"]);
    }
}
