//! Usage window (5h/7d) from the OAuth usage API, behind a 1-minute file cache
//! (impure — reads credentials, hits the network, writes a cache).

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::data::UsageData;
use crate::text::fmt_relative;

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
/// the token.
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
/// into the `Authorization` header in `fetch_usage`, never into the cache or
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

/// Usage window (5h/7d) from the OAuth usage API, behind a 1-minute file cache.
/// Fresh cache → use it; otherwise fetch (refreshing the cache), falling back to
/// a stale cache on any failure. `None` hides the usage rows.
pub(crate) fn gather_usage() -> Option<UsageData> {
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
