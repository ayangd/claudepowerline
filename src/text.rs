//! Pure string / number formatting and truncation helpers.

use chrono::{DateTime, Utc};

use crate::platform;

/// Uppercase the first character and lowercase the rest.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first
            .to_uppercase()
            .chain(chars.flat_map(char::to_lowercase))
            .collect(),
    }
}

/// `"{Effort} {display_name}"`, or just the name when there is no effort level.
pub(crate) fn format_model(effort: &str, name: &str) -> String {
    if effort.is_empty() {
        name.to_string()
    } else {
        format!("{} {}", capitalize(effort), name)
    }
}

/// `"{round(total/1000)}K/{round(limit/1000)}K"`.
pub(crate) fn format_tokens(total: u64, limit: u64) -> String {
    let k = |n: u64| ((n as f64) / 1000.0).round() as i64;
    format!("{}K/{}K", k(total), k(limit))
}

/// Build a shortened-path candidate: the first `head + 1` parts, then `…`, then
/// the last `tail` parts, joined with `/`.
fn cwd_candidate(parts: &[&str], head: usize, tail: usize) -> String {
    let n = parts.len();
    let h = (head + 1).min(n);
    let t = tail.min(n);
    let mut segs: Vec<&str> = parts[..h].to_vec();
    segs.push("…");
    segs.extend_from_slice(&parts[n - t..]);
    segs.join("/")
}

/// Replace `$HOME` with `~`, then middle-truncate with `…` until the path fits
/// within `max` characters: grow the visible head/tail one step past the limit,
/// then back off once.
pub(crate) fn shorten_cwd(raw_cwd: &str, home: &str, max: usize) -> String {
    let mut cwd = if home.is_empty() {
        raw_cwd.to_string()
    } else {
        raw_cwd.replacen(home, "~", 1)
    };

    if cwd.chars().count() > max {
        let parts: Vec<&str> = cwd.split(platform::PATH_SEPARATORS).collect();
        let n = parts.len();
        if n > 3 {
            let mut head = 1usize;
            let mut tail = 1usize;
            let mut cand = cwd_candidate(&parts, head, tail);
            while cand.chars().count() <= max && (head + tail + 1) < n {
                if tail <= head {
                    tail += 1;
                } else {
                    head += 1;
                }
                cand = cwd_candidate(&parts, head, tail);
            }
            if cand.chars().count() > max {
                if tail > 1 {
                    tail -= 1;
                } else {
                    head -= 1;
                }
            }
            cwd = cwd_candidate(&parts, head, tail);
        }
    }
    cwd
}

/// Middle-truncate a branch name to 20 chars (`first10…last9`); the head slice
/// uses an inclusive range.
pub(crate) fn truncate_branch(branch: String) -> String {
    const MAX: usize = 20;
    let chars: Vec<char> = branch.chars().collect();
    let len = chars.len();
    if len <= MAX {
        return branch;
    }
    let half = (MAX - 1) / 2; // 9
    let tail = len - half;
    let first: String = chars[..=half].iter().collect(); // inclusive → 10 chars
    let last: String = chars[tail..].iter().collect(); //   9 chars
    format!("{first}…{last}")
}

/// Bucket a duration in seconds into `Hh Mm` / `Mm Ss` / `Ss`.
pub(crate) fn format_elapsed(secs: i64) -> String {
    if secs >= 3600 {
        format!("{}h{}m", secs / 3600, (secs % 3600) / 60)
    } else if secs >= 60 {
        format!("{}m{}s", secs / 60, secs % 60)
    } else {
        format!("{secs}s")
    }
}

/// Format a reset timestamp (Unix epoch seconds) as a relative countdown
/// `Dd Hh` / `Hh Mm` / `Mm`, `now` if already elapsed, or empty for
/// absent/unrepresentable input.
pub(crate) fn fmt_relative(ts: Option<i64>, now: DateTime<Utc>) -> String {
    let Some(reset) = ts.and_then(|t| DateTime::from_timestamp(t, 0)) else {
        return String::new();
    };
    let diff = (reset - now).num_seconds();
    if diff <= 0 {
        return "now".to_string();
    }
    let (d, h, m) = (diff / 86400, (diff % 86400) / 3600, (diff % 3600) / 60);
    if d > 0 {
        format!("{d}d{h}h")
    } else if h > 0 {
        format!("{h}h{m}m")
    } else {
        format!("{m}m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_with_and_without_effort() {
        assert_eq!(format_model("xhigh", "Opus 4.8"), "Xhigh Opus 4.8");
        assert_eq!(format_model("HIGH", "Opus 4.8"), "High Opus 4.8");
        assert_eq!(format_model("", "Opus 4.8"), "Opus 4.8");
    }

    #[test]
    fn tokens_rounded_to_thousands() {
        assert_eq!(format_tokens(128_000, 1_000_000), "128K/1000K");
        assert_eq!(format_tokens(1_234_567, 200_000), "1235K/200K");
    }

    #[test]
    fn cwd_long_path_middle_truncated() {
        // 31 chars after tilde → grows past 30, then backs off one step.
        assert_eq!(
            shorten_cwd("/home/u/aaaa/bbbb/cccc/dddd/eeee/ffff", "/home/u", 30),
            "~/aaaa/bbbb/cccc/…/eeee/ffff"
        );
    }

    #[test]
    fn cwd_windows_path_truncated() {
        // Backslash-separated paths shorten too (the split accepts both seps).
        assert_eq!(
            shorten_cwd(
                r"C:\Users\u\aaaa\bbbb\cccc\dddd\eeee\ffff",
                r"C:\Users\u",
                30
            ),
            "~/aaaa/bbbb/cccc/…/eeee/ffff"
        );
    }

    #[test]
    fn branch_truncation() {
        assert_eq!(truncate_branch("main".to_string()), "main");
        assert_eq!(
            truncate_branch("feature/some-really-long-branch-name".to_string()),
            "feature/so…anch-name"
        );
    }

    #[test]
    fn elapsed_bucketing() {
        assert_eq!(format_elapsed(0), "0s");
        assert_eq!(format_elapsed(45), "45s");
        assert_eq!(format_elapsed(60), "1m0s");
        assert_eq!(format_elapsed(125), "2m5s");
        assert_eq!(format_elapsed(3600), "1h0m");
        // 7325s = 2h2m5s → hours bucket drops the seconds.
        assert_eq!(format_elapsed(7325), "2h2m");
    }

    #[test]
    fn relative_reset_formatting() {
        let now = DateTime::parse_from_rfc3339("2026-06-25T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let at = |offset_secs: i64| fmt_relative(Some(now.timestamp() + offset_secs), now);
        assert_eq!(fmt_relative(None, now), "");
        assert_eq!(fmt_relative(Some(i64::MAX), now), ""); // unrepresentable
        assert_eq!(at(-60), "now"); // already elapsed
        assert_eq!(at(0), "now"); // exactly now
        assert_eq!(at(45 * 60), "45m");
        assert_eq!(at(2 * 3600 + 30 * 60), "2h30m");
        assert_eq!(at(3 * 86400 + 4 * 3600), "3d4h");
    }
}
