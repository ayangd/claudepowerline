use serde::Deserialize;

/// Mirrors the subset of Claude Code's status-line JSON that the original
/// `statusline.nu` reads from stdin. Every field is optional / defaulted so we
/// degrade gracefully when the harness omits one.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct Input {
    pub(crate) model: Model,
    pub(crate) effort: Effort,
    pub(crate) context_window: ContextWindow,
    pub(crate) cwd: Option<String>,
    pub(crate) transcript_path: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct Model {
    pub(crate) display_name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct Effort {
    pub(crate) level: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct ContextWindow {
    pub(crate) used_percentage: f64,
    pub(crate) total_input_tokens: u64,
    pub(crate) total_output_tokens: u64,
    pub(crate) context_window_size: u64,
}
