//! Elapsed / last-message times and agent-response-latency stats from the
//! transcript JSONL (impure — reads a file and uses the local clock).

use chrono::{DateTime, FixedOffset, Local, Utc};
use serde::Deserialize;

use crate::data::ResponseStats;
use crate::text::format_elapsed;

/// One transcript JSONL record — only the fields we need; the rest are ignored.
#[derive(Debug, Deserialize)]
struct TranscriptEntry {
    #[serde(rename = "type")]
    entry_type: Option<String>,
    timestamp: Option<String>,
}

/// Time stats derived from the transcript in a single pass.
pub(crate) struct TranscriptStats {
    /// First-to-last elapsed wall-clock (`Hh Mm` / `Mm Ss` / `Ss`).
    pub(crate) elapsed: String,
    /// Local `HH:MM` of the last message (empty when unknown).
    pub(crate) last_msg: String,
    /// Time since the last message (`format_elapsed` buckets, clamped ≥0;
    /// empty when unknown).
    pub(crate) last_msg_ago: String,
    /// Agent response-time stats; `None` when there are no completed responses.
    pub(crate) resp: Option<ResponseStats>,
}

impl TranscriptStats {
    /// Pre-transcript defaults: `0s` elapsed, everything else absent.
    fn empty() -> Self {
        Self {
            elapsed: "0s".to_string(),
            last_msg: String::new(),
            last_msg_ago: String::new(),
            resp: None,
        }
    }
}

/// Latencies (seconds, clamped ≥0) from each trigger to the next agent reply.
/// `events` is `(is_assistant, epoch_secs)` in transcript order: a `false` entry
/// (user / tool-result) arms the timer; the next `true` (assistant) records the
/// gap. Overwriting `pending` keeps the *last* trigger before each reply, which
/// is correct for parallel tool-results.
fn response_latencies(events: &[(bool, i64)]) -> Vec<i64> {
    let mut pending: Option<i64> = None;
    let mut out = Vec::new();
    for &(is_assistant, ts) in events {
        if is_assistant {
            if let Some(start) = pending.take() {
                out.push((ts - start).max(0));
            }
        } else {
            pending = Some(ts);
        }
    }
    out
}

/// Nearest-rank percentile of an ascending-sorted slice (`p` in 1..=100).
fn percentile(sorted_asc: &[i64], p: u32) -> i64 {
    let n = sorted_asc.len();
    if n == 0 {
        return 0;
    }
    let rank = ((p as f64 / 100.0) * n as f64).ceil() as usize;
    sorted_asc[rank.saturating_sub(1).min(n - 1)]
}

/// Summarize latencies into formatted response stats, or `None` when empty.
/// `last` preserves recency (original order); avg/percentiles use the sorted set.
fn response_stats(latencies: &[i64]) -> Option<ResponseStats> {
    let &last = latencies.last()?;
    let n = latencies.len();
    let sum: i64 = latencies.iter().sum();
    let avg = (sum as f64 / n as f64).round() as i64;
    let mut sorted = latencies.to_vec();
    sorted.sort_unstable();
    Some(ResponseStats {
        avg: format_elapsed(avg),
        p75: format_elapsed(percentile(&sorted, 75)),
        p90: format_elapsed(percentile(&sorted, 90)),
        p95: format_elapsed(percentile(&sorted, 95)),
        last: format_elapsed(last),
        count: n as u32,
    })
}

/// Parse the transcript JSONL once and derive every time stat. Elapsed/last-msg
/// use only `user`/`assistant` entries with a parseable timestamp; plus the
/// response-latency stats.
pub(crate) fn transcript_stats(transcript_path: Option<&str>) -> TranscriptStats {
    let Some(path) = transcript_path.filter(|p| !p.is_empty()) else {
        return TranscriptStats::empty();
    };
    let Ok(content) = std::fs::read_to_string(path) else {
        return TranscriptStats::empty();
    };

    let events: Vec<(bool, DateTime<FixedOffset>)> = content
        .lines()
        .filter_map(|line| serde_json::from_str::<TranscriptEntry>(line).ok())
        .filter_map(|e| {
            let is_assistant = match e.entry_type.as_deref()? {
                "assistant" => true,
                "user" => false,
                _ => return None,
            };
            let dt = DateTime::parse_from_rfc3339(e.timestamp.as_deref()?).ok()?;
            Some((is_assistant, dt))
        })
        .collect();

    let (Some(&(_, first)), Some(&(_, last))) = (events.first(), events.last()) else {
        return TranscriptStats::empty();
    };

    let elapsed_sec = (last - first).num_seconds().max(0);
    let secs: Vec<(bool, i64)> = events.iter().map(|&(a, dt)| (a, dt.timestamp())).collect();

    let ago_sec = Utc::now().signed_duration_since(last).num_seconds().max(0);

    TranscriptStats {
        elapsed: format_elapsed(elapsed_sec),
        last_msg: last.with_timezone(&Local).format("%H:%M").to_string(),
        last_msg_ago: format_elapsed(ago_sec),
        resp: response_stats(&response_latencies(&secs)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_latency_pairing() {
        let events = [
            (false, 0),  // user prompt
            (true, 5),   // agent reply        -> 5
            (false, 8),  // tool result
            (true, 12),  // agent continues    -> 4
            (false, 20), // parallel result 1
            (false, 22), // parallel result 2  (overwrite -> last)
            (true, 30),  // agent continues    -> 8 (from 22)
            (false, 40), // in-flight prompt, no following assistant
        ];
        assert_eq!(response_latencies(&events), vec![5, 4, 8]);
        // Consecutive assistants: only the first after a trigger counts.
        assert_eq!(
            response_latencies(&[(false, 0), (true, 3), (true, 9)]),
            vec![3]
        );
        // A leading assistant with no pending trigger is ignored.
        assert_eq!(
            response_latencies(&[(true, 1), (false, 2), (true, 5)]),
            vec![3]
        );
        // No triggers -> no latencies.
        assert_eq!(
            response_latencies(&[(true, 1), (true, 2)]),
            Vec::<i64>::new()
        );
    }

    #[test]
    fn percentiles_nearest_rank() {
        let s: Vec<i64> = (1..=10).collect();
        assert_eq!(percentile(&s, 75), 8);
        assert_eq!(percentile(&s, 90), 9);
        assert_eq!(percentile(&s, 95), 10);
        assert_eq!(percentile(&s, 50), 5);
        assert_eq!(percentile(&[42], 95), 42);
    }

    #[test]
    fn response_stats_summary() {
        // latencies [5,4,8]: avg 17/3=5.67->6, last(recency)=8, sorted [4,5,8].
        let r = response_stats(&[5, 4, 8]).unwrap();
        assert_eq!(r.avg, "6s");
        assert_eq!(r.last, "8s");
        assert_eq!(r.count, 3);
        assert_eq!(r.p95, "8s");
        assert!(response_stats(&[]).is_none());
    }
}
