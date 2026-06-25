//! Pure rendering: turn resolved [`StatusData`] into the final ANSI string.

use crate::data::StatusData;
use crate::theme;

/// The status-line renderer strips *all* leading whitespace (ASCII and NBSP), so
/// the percentile line begins with a visible dim continuation marker (not
/// stripped), then non-breaking spaces sized to put `p75` under `avg`. The gap
/// mirrors line 1's `" resp  "` (7 columns); the marker shares the resp icon's
/// East-Asian "ambiguous" width class, so they occupy the same column count.
const RESP_CONT: &str = "\u{21b3}";
const RESP_INDENT: &str = "\u{a0}\u{a0}\u{a0}\u{a0}\u{a0}\u{a0}\u{a0}";

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
        out.push_str(&theme::BRAILLE[8].repeat(full));
    }
    if partial > 0 {
        out.push_str(fg);
        out.push_str(bg_fill);
        out.push_str(theme::BRAILLE[partial]);
    }
    if empty > 0 {
        out.push_str(bg_empty);
        out.push_str(&" ".repeat(empty));
    }
    out.push_str(theme::RESET);
    out
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
    let bar = make_bar(pct, theme::BAR_WIDTH, c, bg, theme::BG_TRACK);
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

    // Lines 2-3: agent response latency (avg + last; percentiles indented).
    if let Some(r) = &data.resp {
        out.push_str(&format!(
            "\n{yellow}{ir} resp{reset}  {white}avg {avg} x{count} · last {last}{reset}",
            yellow = theme::YELLOW,
            ir = theme::ICON_RESP,
            reset = theme::RESET,
            white = theme::WHITE,
            avg = r.avg,
            count = r.count,
            last = r.last,
        ));
        out.push_str(&format!(
            "\n{dim}{cont}{indent}p75 {p75} · p90 {p90} · p95 {p95}{reset}",
            dim = theme::DIM,
            cont = RESP_CONT,
            indent = RESP_INDENT,
            reset = theme::RESET,
            p75 = r.p75,
            p90 = r.p90,
            p95 = r.p95,
        ));
    } else {
        out.push_str(&format!(
            "\n{dim}{ir} resp  avg — · last —{reset}",
            dim = theme::DIM,
            ir = theme::ICON_RESP,
            reset = theme::RESET,
        ));
        out.push_str(&format!(
            "\n{dim}{cont}{indent}p75 — · p90 — · p95 —{reset}",
            dim = theme::DIM,
            cont = RESP_CONT,
            indent = RESP_INDENT,
            reset = theme::RESET,
        ));
    }

    // Context bar + tokens — always shown; absent data renders muted
    // (empty bar + `—`) rather than a fabricated `0%`.
    let (cc, cbar) = match data.context_used {
        Some(used) => {
            let (c, bg) = level_colors(used);
            (c, make_bar(used, theme::BAR_WIDTH, c, bg, theme::BG_TRACK))
        }
        None => (
            theme::DIM,
            make_bar(
                0.0,
                theme::BAR_WIDTH,
                theme::DIM,
                theme::BG_TRACK,
                theme::BG_TRACK,
            ),
        ),
    };
    let pct = match data.context_used {
        Some(used) => format!("{:>3}", used.round() as i64),
        None => format!("{:>3}", "—"),
    };
    let tokens = data.tokens.as_deref().unwrap_or("—");
    out.push_str(&format!(
        "\n{cc}{ic} ctx {cbar} {white}{pct}%{reset} {cyan}{it} {tokens}{reset}",
        ic = theme::ICON_CONTEXT,
        white = theme::WHITE,
        reset = theme::RESET,
        cyan = theme::CYAN,
        it = theme::ICON_TOKENS,
    ));

    // Usage window, when present.
    if let Some(u) = &data.usage {
        out.push_str(&usage_line("5h", u.five_hour, &u.reset_5h));
        out.push_str(&usage_line("7d", u.seven_day, &u.reset_7d));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
