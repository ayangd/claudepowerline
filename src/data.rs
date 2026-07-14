/// Fully-resolved, display-ready status data. `render` turns this into the
/// final ANSI string; all impure resolution happens in `gather`, so this stays
/// deterministic (and free of any credential).
#[derive(Debug, Default, Clone)]
pub struct StatusData {
    pub model: String,
    pub cwd: String,
    /// Empty when not a git repo.
    pub branch: String,
    pub time_elapsed: String,
    /// Empty when there is no transcript / last message.
    pub last_msg: String,
    /// Time since the last message; empty when there is no last message.
    pub last_msg_ago: String,
    /// Agent response-time stats (any trigger → next reply); `None` until there
    /// is at least one completed response.
    pub resp: Option<ResponseStats>,
    /// Context-window usage percentage (0..100); `None` when not yet known
    /// (e.g. first boot) — the renderer shows an empty bar, not a fake `0%`.
    pub context_used: Option<f64>,
    /// Formatted `in+out K / limit K`; `None` when the counts aren't known.
    pub tokens: Option<String>,
    /// Cache composition of the current context; `None` renders `—`
    /// placeholders (unknown before the first API call and after `/compact`).
    pub cache: Option<CacheUsage>,
    /// `None` hides the usage rows entirely.
    pub usage: Option<UsageData>,
}

/// Current-context cache composition: pre-formatted read/write/fresh token
/// counts; `write_share` (0..100, write / (read+write+fresh)) drives the
/// 寫-emphasis color in the renderer.
#[derive(Debug, Default, Clone)]
pub struct CacheUsage {
    pub read: String,
    pub write: String,
    pub fresh: String,
    pub write_share: f64,
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

/// Agent response-time stats, all durations pre-formatted.
#[derive(Debug, Default, Clone)]
pub struct ResponseStats {
    pub avg: String,
    pub p75: String,
    pub p90: String,
    pub p95: String,
    pub last: String,
    pub count: u32,
}
