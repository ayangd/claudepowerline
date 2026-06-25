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
    /// Context-window usage percentage (0..100); `None` when not yet known
    /// (e.g. first boot) — the renderer shows an empty bar, not a fake `0%`.
    pub context_used: Option<f64>,
    /// Formatted `in+out K / limit K`; `None` when the counts aren't known.
    pub tokens: Option<String>,
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
