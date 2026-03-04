use std::sync::OnceLock;

use regex::Regex;

static TIME_REGEX: OnceLock<Regex> = OnceLock::new();

fn time_regex() -> &'static Regex {
    TIME_REGEX.get_or_init(|| {
        Regex::new(r#"(?i)(?:(?P<hour>\d*\.?\d+)h)?(?:(?P<minute>\d*\.?\d+)min)?(?:(?P<second>\d*\.?\d+)s)?"#)
            .unwrap()
    })
}

pub fn parse_time(s: &str) -> f64 {
    let mut seconds = 0.0;
    if let Some(caps) = time_regex().captures(s) {
        if let Some(hour) = caps.get(1) {
            let hour: f64 = hour.as_str().parse().unwrap_or(0.0);
            seconds += hour * 3600.0;
        }
        if let Some(minute) = caps.get(2) {
            let minute: f64 = minute.as_str().parse().unwrap_or(0.0);
            seconds += minute * 60.0;
        }
        if let Some(second) = caps.get(3) {
            let second: f64 = second.as_str().parse().unwrap_or(0.0);
            seconds += second;
        }
    }
    seconds
}

pub fn format_time(seconds: f64) -> String {
    let hours = (seconds / 3600.0).floor();
    let minutes = ((seconds % 3600.0) / 60.0).floor();
    let seconds = seconds % 60.0;
    let mut format_str = String::new();
    if hours > 0.0 {
        format_str += &format!("{}h", hours as u64);
    }
    if minutes > 0.0 {
        format_str += &format!("{}min", minutes as u64);
    }
    if seconds > 0.0 || format_str.is_empty() {
        format_str += &format!("{}s", seconds);
    }
    format_str
}
