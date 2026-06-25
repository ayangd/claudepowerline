//! Resolve raw status-line JSON + the environment into display-ready
//! [`StatusData`]. This is the impure orchestrator: it drives git, the
//! transcript, and the usage window.

use crate::data::StatusData;
use crate::git::git_branch;
use crate::input::Input;
use crate::text::{format_model, format_tokens, shorten_cwd};
use crate::transcript::transcript_times;
use crate::usage::gather_usage;

/// Resolve everything `render` needs from a parsed [`Input`]: model string,
/// shortened cwd, git branch, token counts, transcript times, and the usage
/// window.
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

/// Parse the status-line JSON in `raw` and gather everything `render` needs.
pub fn gather_from_json(raw: &str) -> Result<StatusData, serde_json::Error> {
    Ok(gather(&serde_json::from_str::<Input>(raw)?))
}
