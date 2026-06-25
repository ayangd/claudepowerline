//! claudepowerline — a Claude Code statusline written in Rust.
//!
//! The impure work (env, git, filesystem, clock, network) lives in `gather`;
//! turning the resolved [`StatusData`] into the final ANSI string lives in the
//! pure `render`. Keeping that boundary lets the renderer be golden-tested
//! deterministically — and keeps anything secret (the OAuth token used to fetch
//! the usage window) out of the rendered output entirely.

mod data;
mod gather;
mod git;
mod input;
mod render;
mod text;
mod theme;
mod transcript;
mod usage;

pub use data::{ResponseStats, StatusData, UsageData};
pub use gather::gather_from_json;
pub use render::render;
