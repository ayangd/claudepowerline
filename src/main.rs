use std::io::Read;

use serde::Deserialize;

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

fn main() {
    let mut raw = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut raw) {
        eprintln!("claudepowerline: failed to read stdin: {e}");
        std::process::exit(1);
    }

    let input: Input = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(e) => {
            eprintln!("claudepowerline: invalid status-line JSON: {e}");
            std::process::exit(1);
        }
    };

    let model = input.model.display_name.as_deref().unwrap_or("Unknown");
    let cwd = input.cwd.as_deref().unwrap_or("?");
    let pct = input.context_window.used_percentage;

    // Placeholder render — real powerline segments (git, bars, usage window)
    // come next, ported from statusline.nu.
    print!("{model}  {cwd}  ctx {pct:.0}%");
}
