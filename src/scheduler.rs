use crate::tray::TrayAction;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

static LAST_TRIGGER: OnceLock<AtomicI64> = OnceLock::new();

fn last_trigger() -> &'static AtomicI64 {
    LAST_TRIGGER.get_or_init(|| AtomicI64::new(0))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScheduleMode {
    SmartCapture,
    Recording,
    Clip,
    ScrollCapture,
}

struct Config {
    enabled: bool,
    mode: ScheduleMode,
    interval_minutes: u64,
    start_hour: u32,
    start_min: u32,
    end_hour: u32,
    end_min: u32,
    max_count: u64,
}

fn read_config() -> Config {
    let enabled = std::env::var("PARKER_SCHEDULE_ENABLED")
        .ok()
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes"))
        .unwrap_or(false);

    let mode = match std::env::var("PARKER_SCHEDULE_MODE")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "recording" => ScheduleMode::Recording,
        "clip" => ScheduleMode::Clip,
        "scroll-capture" | "scroll" => ScheduleMode::ScrollCapture,
        _ => ScheduleMode::SmartCapture,
    };

    let interval_minutes = std::env::var("PARKER_SCHEDULE_INTERVAL")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v >= 1)
        .unwrap_or(30);

    let (start_hour, start_min) = parse_time("PARKER_SCHEDULE_START", 0, 0);
    let (end_hour, end_min) = parse_time("PARKER_SCHEDULE_END", 23, 59);

    let max_count = std::env::var("PARKER_SCHEDULE_COUNT")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(u64::MAX);

    Config {
        enabled,
        mode,
        interval_minutes,
        start_hour,
        start_min,
        end_hour,
        end_min,
        max_count,
    }
}

fn parse_time(key: &str, default_hour: u32, default_min: u32) -> (u32, u32) {
    let val = std::env::var(key).unwrap_or_default();
    let parts: Vec<&str> = val.split(':').collect();
    if parts.len() != 2 {
        return (default_hour, default_min);
    }
    let h = parts[0].parse::<u32>().ok().filter(|h| *h < 24).unwrap_or(default_hour);
    let m = parts[1].parse::<u32>().ok().filter(|m| *m < 60).unwrap_or(default_min);
    (h, m)
}

fn minutes_since_midnight() -> u32 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let hours = (secs / 3600) % 24;
    let mins = (secs / 60) % 60;
    (hours as u32) * 60 + mins as u32
}

fn unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[allow(dead_code)]
pub fn get_status() -> String {
    let cfg = read_config();
    if !cfg.enabled {
        return "Schedule: disabled".to_string();
    }
    let mode_str = match cfg.mode {
        ScheduleMode::SmartCapture => "smart capture",
        ScheduleMode::Recording => "recording",
        ScheduleMode::Clip => "clip",
        ScheduleMode::ScrollCapture => "scroll capture",
    };
    let count = last_trigger().load(Ordering::Relaxed);
    let remaining = if count > 0 {
        let elapsed = unix_timestamp() - count;
        let next = cfg.interval_minutes.saturating_mul(60).saturating_sub(elapsed as u64);
        if next == 0 {
            "now".to_string()
        } else {
            format!("{next}s")
        }
    } else {
        "ready".to_string()
    };
    format!("Schedule: {mode_str} every {}m (next in {remaining})", cfg.interval_minutes)
}

pub fn check_schedule(busy: bool) -> Option<TrayAction> {
    let cfg = read_config();
    if !cfg.enabled || busy {
        return None;
    }

    if last_trigger().load(Ordering::Relaxed) >= cfg.max_count as i64 {
        return None;
    }

    let now_min = minutes_since_midnight();
    let start_min = cfg.start_hour * 60 + cfg.start_min;
    let end_min = cfg.end_hour * 60 + cfg.end_min;
    if now_min < start_min || now_min > end_min {
        return None;
    }

    let last = last_trigger().load(Ordering::Relaxed);
    let interval_secs = cfg.interval_minutes * 60;
    if last > 0 && unix_timestamp() - last < interval_secs as i64 {
        return None;
    }

    last_trigger().fetch_add(1, Ordering::Relaxed);
    let _ = std::fs::write(
        std::env::temp_dir().join("parker-last-trigger.txt"),
        unix_timestamp().to_string(),
    );

    Some(match cfg.mode {
        ScheduleMode::SmartCapture => TrayAction::SmartCapture,
        ScheduleMode::Recording => TrayAction::ToggleRecording,
        ScheduleMode::Clip => TrayAction::ToggleClipRecording,
        ScheduleMode::ScrollCapture => TrayAction::ToggleScrollCapture,
    })
}
