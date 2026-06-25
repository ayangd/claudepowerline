//! Elapsed / last-message times from the transcript JSONL (impure — reads a
//! file and uses the local clock).

use chrono::{DateTime, Local};
use serde::Deserialize;

use crate::text::format_elapsed;

/// One transcript JSONL record — only the fields we need; the rest are ignored.
#[derive(Debug, Deserialize)]
struct TranscriptEntry {
    #[serde(rename = "type")]
    entry_type: Option<String>,
    timestamp: Option<String>,
}

/// First-to-last elapsed time and the local wall-clock of the last message,
/// read from the transcript JSONL. Mirrors statusline.nu: only `user` and
/// `assistant` entries count, and both endpoints need a parseable timestamp —
/// otherwise the defaults (`0s`, empty) stand.
pub(crate) fn transcript_times(transcript_path: Option<&str>) -> (String, String) {
    let default = || ("0s".to_string(), String::new());

    let Some(path) = transcript_path.filter(|p| !p.is_empty()) else {
        return default();
    };
    let Ok(content) = std::fs::read_to_string(path) else {
        return default();
    };

    let stamps: Vec<Option<String>> = content
        .lines()
        .filter_map(|line| serde_json::from_str::<TranscriptEntry>(line).ok())
        .filter(|e| matches!(e.entry_type.as_deref(), Some("user") | Some("assistant")))
        .map(|e| e.timestamp)
        .collect();

    let (Some(first), Some(last)) = (stamps.first(), stamps.last()) else {
        return default();
    };
    let first_ts = first.as_deref().unwrap_or("");
    let last_ts = last.as_deref().unwrap_or("");
    if first_ts.is_empty() || last_ts.is_empty() {
        return default();
    }

    let (Ok(first_dt), Ok(last_dt)) = (
        DateTime::parse_from_rfc3339(first_ts),
        DateTime::parse_from_rfc3339(last_ts),
    ) else {
        return default();
    };

    let elapsed_sec = (last_dt - first_dt).num_seconds().max(0);
    let last_msg = last_dt.with_timezone(&Local).format("%H:%M").to_string();
    (format_elapsed(elapsed_sec), last_msg)
}
