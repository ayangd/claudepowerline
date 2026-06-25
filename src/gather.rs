//! Resolve raw status-line JSON + the environment into display-ready
//! [`StatusData`]. This is the impure orchestrator: it drives git, the
//! transcript, and the usage window.

use crate::data::StatusData;
use crate::git::git_branch;
use crate::input::Input;
use crate::text::{format_model, format_tokens, shorten_cwd};
use crate::transcript::transcript_stats;
use crate::usage::gather_usage;

/// Resolve everything `render` needs from a parsed [`Input`]: model string,
/// shortened cwd, git branch, token counts, transcript times, and the usage
/// window.
fn gather(input: &Input) -> StatusData {
    let home = std::env::var("HOME").unwrap_or_default();

    let model = format_model(
        input
            .effort
            .as_ref()
            .and_then(|e| e.level.as_deref())
            .unwrap_or(""),
        input
            .model
            .as_ref()
            .and_then(|m| m.display_name.as_deref())
            .unwrap_or("Unknown"),
    );

    // Keep the real path for git; show a tilde'd, shortened one.
    let raw_cwd = input.cwd.clone().unwrap_or_else(|| home.clone());
    let cwd = shorten_cwd(&raw_cwd, &home, 30);
    let branch = git_branch(&raw_cwd);

    // Context numbers are absent on first boot; keep them `None` (the renderer
    // shows an empty bar) rather than faking a `0%` / `0K`.
    let cw = input.context_window.as_ref();
    let context_used = cw.and_then(|c| c.used_percentage);
    let tokens = cw.and_then(|c| {
        match (
            c.total_input_tokens,
            c.total_output_tokens,
            c.context_window_size,
        ) {
            (Some(i), Some(o), Some(s)) => Some(format_tokens(i + o, s)),
            _ => None,
        }
    });

    let stats = transcript_stats(input.transcript_path.as_deref());

    StatusData {
        model,
        cwd,
        branch,
        time_elapsed: stats.elapsed,
        last_msg: stats.last_msg,
        resp: stats.resp,
        context_used,
        tokens,
        usage: gather_usage(),
    }
}

/// Parse the status-line JSON in `raw` and gather everything `render` needs.
pub fn gather_from_json(raw: &str) -> Result<StatusData, serde_json::Error> {
    Ok(gather(&serde_json::from_str::<Input>(raw)?))
}
