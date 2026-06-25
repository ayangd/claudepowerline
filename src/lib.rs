//! claudepowerline — a Claude Code statusline, ported from `statusline.nu`.
//!
//! The impure work (env, git, filesystem, clock, network) lives in [`gather`];
//! turning the resolved [`StatusData`] into the final ANSI string lives in the
//! pure [`render`]. Keeping that boundary lets the renderer be golden-tested
//! deterministically — and keeps anything secret (the OAuth token used to fetch
//! the usage window) out of the rendered output entirely.

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};

/// Mirrors the subset of Claude Code's status-line JSON that the original
/// `statusline.nu` reads from stdin. Every field is optional / defaulted so we
/// degrade gracefully when the harness omits one.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Input {
    model: Model,
    effort: Effort,
    context_window: ContextWindow,
    cwd: Option<String>,
    transcript_path: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Model {
    display_name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Effort {
    level: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ContextWindow {
    used_percentage: f64,
    total_input_tokens: u64,
    total_output_tokens: u64,
    context_window_size: u64,
}

/// Uppercase the first character and lowercase the rest, matching Nushell's
/// `str capitalize`.
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
fn format_model(effort: &str, name: &str) -> String {
    if effort.is_empty() {
        name.to_string()
    } else {
        format!("{} {}", capitalize(effort), name)
    }
}

/// `"{round(total/1000)}K/{round(limit/1000)}K"`.
fn format_tokens(total: u64, limit: u64) -> String {
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
/// within `max` characters. Faithful port of the `statusline.nu` cwd logic:
/// grow the visible head/tail one step past the limit, then back off once.
fn shorten_cwd(raw_cwd: &str, home: &str, max: usize) -> String {
    let mut cwd = if home.is_empty() {
        raw_cwd.to_string()
    } else {
        raw_cwd.replacen(home, "~", 1)
    };

    if cwd.chars().count() > max {
        let parts: Vec<&str> = cwd.split('/').collect();
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

/// Middle-truncate a branch name to 20 chars (`first10…last9`), matching the
/// Nushell `str substring` (inclusive-range) behaviour.
fn truncate_branch(branch: String) -> String {
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

/// Current git branch for `raw_cwd`; detached HEAD shows `:<short-hash>`.
/// Any failure (not a repo, git missing) yields an empty string.
fn git_branch(raw_cwd: &str) -> String {
    let Ok(out) = std::process::Command::new("git")
        .args(["-C", raw_cwd, "branch", "--show-current"])
        .output()
    else {
        return String::new();
    };
    if !out.status.success() {
        return String::new();
    }

    let mut branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if branch.is_empty() {
        // Detached HEAD — fall back to the short commit hash.
        if let Ok(ha) = std::process::Command::new("git")
            .args(["-C", raw_cwd, "rev-parse", "--short", "HEAD"])
            .output()
            && ha.status.success()
        {
            branch = format!(":{}", String::from_utf8_lossy(&ha.stdout).trim());
        }
    }
    truncate_branch(branch)
}

const BAR_WIDTH: usize = 20;

/// Braille fill levels, bottom→top: blank … full (8 vertical sub-steps).
const BRAILLE: [&str; 9] = ["⠀", "⡀", "⡄", "⡆", "⡇", "⣇", "⣧", "⣷", "⣿"];

/// ANSI palette + nerd-font icons ported from statusline.nu.
mod theme {
    // Foreground colors (Nushell `ansi` names noted alongside).
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m"; // attr_bold
    pub const DIM: &str = "\x1b[2m"; // attr_dimmed
    pub const BLUE: &str = "\x1b[94m"; // light_blue
    pub const GREEN: &str = "\x1b[92m"; // light_green
    pub const YELLOW: &str = "\x1b[93m"; // light_yellow
    pub const RED: &str = "\x1b[91m"; // light_red
    pub const CYAN: &str = "\x1b[96m"; // light_cyan
    pub const MAGENTA: &str = "\x1b[95m"; // light_magenta
    pub const ORANGE: &str = "\x1b[33m"; // ansi yellow, used as orange
    pub const WHITE: &str = "\x1b[37m";

    // 256-color bar backgrounds: tinted fill + dark track.
    pub const BG_TRACK: &str = "\x1b[48;5;236m";
    pub const BG_GREEN: &str = "\x1b[48;5;22m";
    pub const BG_ORANGE: &str = "\x1b[48;5;94m";
    pub const BG_RED: &str = "\x1b[48;5;52m";

    // Nerd-font icons.
    pub const ICON_MODEL: &str = "󰚩";
    pub const ICON_FOLDER: &str = "󰉋";
    pub const ICON_CONTEXT: &str = "󰘚";
    pub const ICON_TOKENS: &str = "󰦨";
    pub const ICON_TIME: &str = "󱑆";
    pub const ICON_USAGE: &str = "󰄪";
    pub const ICON_GIT: &str = "󰘬";
    pub const ICON_RESET: &str = "󱫤";
}

/// Cell counts `(full, partial, empty)` for a `width`-cell bar at `pct` percent.
/// Each cell is 8 vertical sub-steps; `partial` is 0 (no partial cell) or the
/// braille level index 1..=7. Input is clamped so an over-100% value can't
/// underflow the empty count.
fn bar_cells(pct: f64, width: usize) -> (usize, usize, usize) {
    let total_steps = width * 8;
    let filled = ((pct / 100.0) * total_steps as f64).round() as i64;
    let filled = filled.clamp(0, total_steps as i64) as usize;
    let full = filled / 8;
    let partial = filled % 8;
    let has_partial = usize::from(partial > 0);
    let empty = width.saturating_sub(full).saturating_sub(has_partial);
    (full, partial, empty)
}

/// Render a braille progress bar: `fg` braille dots on `bg_fill` for the filled
/// portion, spaces on `bg_empty` for the track, then reset.
fn make_bar(pct: f64, width: usize, fg: &str, bg_fill: &str, bg_empty: &str) -> String {
    let (full, partial, empty) = bar_cells(pct, width);
    let mut out = String::new();
    if full > 0 {
        out.push_str(fg);
        out.push_str(bg_fill);
        out.push_str(&BRAILLE[8].repeat(full));
    }
    if partial > 0 {
        out.push_str(fg);
        out.push_str(bg_fill);
        out.push_str(BRAILLE[partial]);
    }
    if empty > 0 {
        out.push_str(bg_empty);
        out.push_str(&" ".repeat(empty));
    }
    out.push_str(theme::RESET);
    out
}

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
fn transcript_times(transcript_path: Option<&str>) -> (String, String) {
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

/// Bucket a duration in seconds into `Hh Mm` / `Mm Ss` / `Ss`, matching the
/// statusline.nu elapsed-time format.
fn format_elapsed(secs: i64) -> String {
    if secs >= 3600 {
        format!("{}h{}m", secs / 3600, (secs % 3600) / 60)
    } else if secs >= 60 {
        format!("{}m{}s", secs / 60, secs % 60)
    } else {
        format!("{secs}s")
    }
}

/// Fully-resolved, display-ready status data. [`render`] turns this into the
/// final ANSI string; all impure resolution happens in [`gather`], so this
/// stays deterministic (and free of any credential).
#[derive(Debug, Default, Clone)]
pub struct StatusData {
    pub model: String,
    pub cwd: String,
    /// Empty when not a git repo.
    pub branch: String,
    pub time_elapsed: String,
    /// Empty when there is no transcript / last message.
    pub last_msg: String,
    /// Context-window usage percentage (0..100).
    pub context_used: f64,
    pub tokens: String,
    /// `None` hides the usage rows entirely.
    pub usage: Option<UsageData>,
}

/// 5h / 7d usage-window utilization, with pre-formatted reset countdowns
/// (an empty string hides a countdown).
#[derive(Debug, Default, Clone)]
pub struct UsageData {
    pub five_hour: f64,
    pub seven_day: f64,
    pub reset_5h: String,
    pub reset_7d: String,
}

/// Threshold colors `(fg, bg)` for a percentage: >80 red, >60 orange, else green.
fn level_colors(pct: f64) -> (&'static str, &'static str) {
    if pct > 80.0 {
        (theme::RED, theme::BG_RED)
    } else if pct > 60.0 {
        (theme::ORANGE, theme::BG_ORANGE)
    } else {
        (theme::GREEN, theme::BG_GREEN)
    }
}

/// One usage row (`5h` / `7d`): colored bar, right-aligned percentage, and an
/// optional reset countdown.
fn usage_line(label: &str, pct: f64, reset_rel: &str) -> String {
    let (c, bg) = level_colors(pct);
    let bar = make_bar(pct, BAR_WIDTH, c, bg, theme::BG_TRACK);
    let p = pct.round() as i64;
    let mut line = format!(
        "\n{c}{iu} {label}  {bar} {white}{p:>3}%{reset}",
        iu = theme::ICON_USAGE,
        white = theme::WHITE,
        reset = theme::RESET,
    );
    if !reset_rel.is_empty() {
        line.push_str(&format!(
            " {c}{ir} {reset_rel}{reset}",
            ir = theme::ICON_RESET,
            reset = theme::RESET,
        ));
    }
    line
}

/// Render the final status string (2–4 lines) from resolved data.
pub fn render(data: &StatusData) -> String {
    // Line 1: model · cwd · git · time · last-msg.
    let mut out = format!(
        "{bold}{blue}{im} {model}{reset} {cyan}{ifol} {cwd}{reset}",
        bold = theme::BOLD,
        blue = theme::BLUE,
        im = theme::ICON_MODEL,
        reset = theme::RESET,
        cyan = theme::CYAN,
        ifol = theme::ICON_FOLDER,
        model = data.model,
        cwd = data.cwd,
    );
    if !data.branch.is_empty() {
        out.push_str(&format!(
            " {magenta}{ig} {branch}{reset}",
            magenta = theme::MAGENTA,
            ig = theme::ICON_GIT,
            reset = theme::RESET,
            branch = data.branch,
        ));
    }
    out.push_str(&format!(
        " {yellow}{itime} {time}{reset}",
        yellow = theme::YELLOW,
        itime = theme::ICON_TIME,
        reset = theme::RESET,
        time = data.time_elapsed,
    ));
    if !data.last_msg.is_empty() {
        out.push_str(&format!(
            " {dim}後 {last}{reset}",
            dim = theme::DIM,
            reset = theme::RESET,
            last = data.last_msg,
        ));
    }

    // Line 2: context bar + tokens.
    let (cc, cbg) = level_colors(data.context_used);
    let cbar = make_bar(data.context_used, BAR_WIDTH, cc, cbg, theme::BG_TRACK);
    let pct = data.context_used.round() as i64;
    out.push_str(&format!(
        "\n{cc}{ic} ctx {cbar} {white}{pct:>3}%{reset} {cyan}{it} {tokens}{reset}",
        ic = theme::ICON_CONTEXT,
        white = theme::WHITE,
        reset = theme::RESET,
        cyan = theme::CYAN,
        it = theme::ICON_TOKENS,
        tokens = data.tokens,
    ));

    // Lines 3-4: usage window, when present.
    if let Some(u) = &data.usage {
        out.push_str(&usage_line("5h", u.five_hour, &u.reset_5h));
        out.push_str(&usage_line("7d", u.seven_day, &u.reset_7d));
    }

    out
}

/// Resolve everything [`render`] needs from a parsed [`Input`]: model string,
/// shortened cwd, git branch, token counts, transcript times, and (unit #7) the
/// usage window.
fn gather(input: &Input) -> StatusData {
    let home = std::env::var("HOME").unwrap_or_default();

    let model = format_model(
        input.effort.level.as_deref().unwrap_or(""),
        input.model.display_name.as_deref().unwrap_or("Unknown"),
    );

    // Keep the real path for git; show a tilde'd, shortened one.
    let raw_cwd = input.cwd.clone().unwrap_or_else(|| home.clone());
    let cwd = shorten_cwd(&raw_cwd, &home, 30);
    let branch = git_branch(&raw_cwd);

    let cw = &input.context_window;
    let tokens = format_tokens(
        cw.total_input_tokens + cw.total_output_tokens,
        cw.context_window_size,
    );

    let (time_elapsed, last_msg) = transcript_times(input.transcript_path.as_deref());

    StatusData {
        model,
        cwd,
        branch,
        time_elapsed,
        last_msg,
        context_used: cw.used_percentage,
        tokens,
        usage: gather_usage(),
    }
}

// ---- Usage window (5h/7d) -------------------------------------------------

/// Anthropic OAuth usage API response (only the fields we read).
#[derive(Debug, Deserialize)]
struct UsageResponse {
    five_hour: Option<UsageWindow>,
    seven_day: Option<UsageWindow>,
}

#[derive(Debug, Default, Deserialize)]
struct UsageWindow {
    utilization: Option<f64>,
    resets_at: Option<String>,
}

/// On-disk usage cache: utilization percentages + raw reset timestamps — never
/// the token. Mirrors the keys statusline.nu writes.
#[derive(Debug, Default, Serialize, Deserialize)]
struct UsageCache {
    five_hour_pct: f64,
    seven_day_pct: f64,
    five_hour_reset: String,
    seven_day_reset: String,
}

/// `~/.claude/.credentials.json` → `claudeAiOauth.accessToken`.
#[derive(Debug, Deserialize)]
struct Credentials {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: Option<OauthCreds>,
}

#[derive(Debug, Deserialize)]
struct OauthCreds {
    #[serde(rename = "accessToken")]
    access_token: Option<String>,
}

const USAGE_API: &str = "https://api.anthropic.com/api/oauth/usage";
const USAGE_CACHE_MAX_AGE_SECS: u64 = 60;

/// Cache-file age in seconds, or `None` if it is absent / can't be stat'd.
fn cache_age_secs(path: &std::path::Path) -> Option<u64> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    modified.elapsed().ok().map(|d| d.as_secs())
}

fn read_usage_cache(path: &std::path::Path) -> Option<UsageCache> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// Best-effort cache write — failures are silently ignored.
fn write_usage_cache(path: &std::path::Path, cache: &UsageCache) {
    let Ok(json) = serde_json::to_string(cache) else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, json);
}

/// Read the OAuth access token. Confined to this function — it only ever flows
/// into the `Authorization` header in [`fetch_usage`], never into the cache or
/// the rendered output.
fn read_access_token(home: &str) -> Option<String> {
    let path = std::path::Path::new(home).join(".claude/.credentials.json");
    let creds: Credentials = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    creds.claude_ai_oauth?.access_token
}

/// Fetch the usage window from the OAuth API (5s timeout), shaped for the cache.
/// A missing `utilization` becomes the `-1` "absent" sentinel.
fn fetch_usage(home: &str) -> Option<UsageCache> {
    let token = read_access_token(home)?;
    if token.is_empty() {
        return None;
    }

    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(5))
        .build();
    let body = agent
        .get(USAGE_API)
        .set("Authorization", &format!("Bearer {token}"))
        .set("anthropic-beta", "oauth-2025-04-20")
        .call()
        .ok()?
        .into_string()
        .ok()?;

    let resp: UsageResponse = serde_json::from_str(&body).ok()?;
    let window = |w: Option<UsageWindow>| {
        let w = w.unwrap_or_default();
        (
            w.utilization.unwrap_or(-1.0),
            w.resets_at.unwrap_or_default(),
        )
    };
    let (five_hour_pct, five_hour_reset) = window(resp.five_hour);
    let (seven_day_pct, seven_day_reset) = window(resp.seven_day);
    Some(UsageCache {
        five_hour_pct,
        seven_day_pct,
        five_hour_reset,
        seven_day_reset,
    })
}

/// Format an RFC-3339 reset timestamp as a relative countdown `Dd Hh` / `Hh Mm`
/// / `Mm`, `now` if already elapsed, or empty for empty/unparseable input.
fn fmt_relative(ts: &str, now: DateTime<Utc>) -> String {
    if ts.is_empty() {
        return String::new();
    }
    let Ok(reset) = DateTime::parse_from_rfc3339(ts) else {
        return String::new();
    };
    let diff = (reset.with_timezone(&Utc) - now).num_seconds();
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

/// Usage window (5h/7d) from the OAuth usage API, behind a 1-minute file cache.
/// Fresh cache → use it; otherwise fetch (refreshing the cache), falling back to
/// a stale cache on any failure. `None` hides the usage rows.
fn gather_usage() -> Option<UsageData> {
    let home = std::env::var("HOME").ok()?;
    let cache_path = std::path::Path::new(&home).join(".claude/cache/usage-window.json");

    let fresh = cache_age_secs(&cache_path).is_some_and(|age| age < USAGE_CACHE_MAX_AGE_SECS);
    let raw = if fresh {
        read_usage_cache(&cache_path)?
    } else {
        match fetch_usage(&home) {
            Some(c) => {
                write_usage_cache(&cache_path, &c);
                c
            }
            None => read_usage_cache(&cache_path)?,
        }
    };

    if raw.five_hour_pct < 0.0 {
        return None;
    }

    let now = Utc::now();
    Some(UsageData {
        five_hour: raw.five_hour_pct,
        seven_day: raw.seven_day_pct,
        reset_5h: fmt_relative(&raw.five_hour_reset, now),
        reset_7d: fmt_relative(&raw.seven_day_reset, now),
    })
}

/// Parse the status-line JSON in `raw` and gather everything [`render`] needs.
pub fn gather_from_json(raw: &str) -> Result<StatusData, serde_json::Error> {
    Ok(gather(&serde_json::from_str::<Input>(raw)?))
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
    fn branch_truncation() {
        assert_eq!(truncate_branch("main".to_string()), "main");
        assert_eq!(
            truncate_branch("feature/some-really-long-branch-name".to_string()),
            "feature/so…anch-name"
        );
    }

    #[test]
    fn bar_fill_math() {
        assert_eq!(bar_cells(0.0, 20), (0, 0, 20));
        assert_eq!(bar_cells(50.0, 20), (10, 0, 10));
        assert_eq!(bar_cells(100.0, 20), (20, 0, 0));
        // 42% → 67/160 steps → 8 full cells, level-3 partial, 11 empty.
        assert_eq!(bar_cells(42.0, 20), (8, 3, 11));
        // Over 100% clamps to a full bar (no empty-count underflow).
        assert_eq!(bar_cells(150.0, 20), (20, 0, 0));
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
        let at = |s: &str| fmt_relative(s, now);
        assert_eq!(at(""), "");
        assert_eq!(at("garbage"), "");
        assert_eq!(at("2026-06-25T11:59:00Z"), "now"); // already elapsed
        assert_eq!(at("2026-06-25T12:00:00Z"), "now"); // exactly now
        assert_eq!(at("2026-06-25T12:45:00Z"), "45m");
        assert_eq!(at("2026-06-25T14:30:00Z"), "2h30m");
        assert_eq!(at("2026-06-28T16:00:00Z"), "3d4h");
    }
}
