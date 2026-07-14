//! Golden snapshot tests for the pure `render`. Each case is a hand-authored,
//! synthetic `StatusData` — gather never runs here, so no credential, real
//! transcript, or network response is ever involved. Regenerate the goldens
//! with `UPDATE_GOLDEN=1 cargo test --test golden`.

use std::path::PathBuf;

use claudepowerline::{CacheUsage, ResponseStats, StatusData, UsageData, render};

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

/// Compare `render(data)` to `tests/golden/<name>.txt`, or rewrite it when
/// `UPDATE_GOLDEN` is set. Every comparison also runs the credential guard.
fn check_golden(name: &str, data: &StatusData) {
    let actual = render(data);
    assert_no_credentials(&actual);

    let path = golden_dir().join(format!("{name}.txt"));
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(golden_dir()).unwrap();
        std::fs::write(&path, &actual).unwrap();
        return;
    }

    let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("missing golden '{name}' ({e}); regenerate with UPDATE_GOLDEN=1")
    });
    assert_eq!(
        actual, expected,
        "golden mismatch for '{name}' (UPDATE_GOLDEN=1 to refresh)"
    );
}

/// Belt-and-suspenders: the rendered status line must never contain anything
/// credential-shaped. `render` takes no token, so this guards against a future
/// change wiring a secret into a segment.
fn assert_no_credentials(rendered: &str) {
    for needle in [
        "Bearer",
        "sk-ant",
        "accessToken",
        "refreshToken",
        "claudeAiOauth",
    ] {
        assert!(
            !rendered.contains(needle),
            "rendered output unexpectedly contains credential marker {needle:?}"
        );
    }
}

fn full_fixture() -> StatusData {
    StatusData {
        model: "Xhigh Opus 4.8".into(),
        cwd: "~/projects/claudepowerline".into(),
        branch: "main".into(),
        time_elapsed: "5m30s".into(),
        last_msg: "17:05".into(),
        last_msg_ago: "32s".into(),
        resp: Some(ResponseStats {
            avg: "14s".into(),
            p75: "20s".into(),
            p90: "35s".into(),
            p95: "50s".into(),
            last: "9s".into(),
            count: 29,
        }),
        context_used: Some(42.0),
        tokens: Some("128K/1000K".into()),
        cache: Some(CacheUsage {
            read: "82K".into(),
            write: "5K".into(),
            fresh: "9K".into(),
            write_share: 5.2,
        }),
        usage: Some(UsageData {
            five_hour: 30.0,
            seven_day: 55.0,
            reset_5h: "2h15m".into(),
            reset_7d: "3d4h".into(),
        }),
    }
}

#[test]
fn golden_full() {
    // All segments present: git, last-msg, usage with both resets shown.
    check_golden("full", &full_fixture());
}

#[test]
fn golden_high_thresholds() {
    // context red (>80); 5h red with reset shown; 7d orange with reset omitted.
    let data = StatusData {
        model: "High Sonnet 4.6".into(),
        cwd: "~/work/api".into(),
        branch: "release/2026-06".into(),
        time_elapsed: "1h2m".into(),
        last_msg: "09:13".into(),
        last_msg_ago: "4m2s".into(),
        resp: Some(ResponseStats {
            avg: "20s".into(),
            p75: "30s".into(),
            p90: "55s".into(),
            p95: "1m10s".into(),
            last: "8s".into(),
            count: 50,
        }),
        context_used: Some(85.0),
        tokens: Some("170K/200K".into()),
        cache: Some(CacheUsage {
            read: "2K".into(),
            write: "160K".into(),
            fresh: "8K".into(),
            write_share: 94.1,
        }),
        usage: Some(UsageData {
            five_hour: 92.0,
            seven_day: 65.0,
            reset_5h: "12m".into(),
            reset_7d: String::new(),
        }),
    };
    check_golden("high_thresholds", &data);
}

#[test]
fn golden_minimal() {
    // No git, no last-msg, no usage rows.
    let data = StatusData {
        model: "Opus 4.8".into(),
        cwd: "/tmp".into(),
        branch: String::new(),
        time_elapsed: "0s".into(),
        last_msg: String::new(),
        last_msg_ago: String::new(),
        resp: None,
        context_used: Some(5.0),
        tokens: Some("0K/1000K".into()),
        cache: None,
        usage: None,
    };
    check_golden("minimal", &data);
}

#[test]
fn golden_first_boot() {
    // First session boot: context numbers not known yet → empty bar + `—`
    // readouts, but line 2 stays visible.
    let data = StatusData {
        model: "Opus 4.8".into(),
        cwd: "~/projects/claudepowerline".into(),
        branch: "main".into(),
        time_elapsed: "0s".into(),
        last_msg: String::new(),
        last_msg_ago: String::new(),
        resp: None,
        context_used: None,
        tokens: None,
        cache: None,
        usage: None,
    };
    check_golden("first_boot", &data);
}

#[test]
fn render_never_leaks_credentials() {
    // The richest fixture (usage + resets present) must still be clean.
    assert_no_credentials(&render(&full_fixture()));
}
