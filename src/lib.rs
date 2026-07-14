//! claudepowerline — a Claude Code statusline written in Rust.
//!
//! The impure work (env, git, filesystem, clock) lives in `gather`; turning the
//! resolved [`StatusData`] into the final ANSI string lives in the pure
//! `render`. Keeping that boundary lets the renderer be golden-tested
//! deterministically.

mod data;
mod gather;
mod git;
mod input;
mod platform;
mod render;
mod text;
mod theme;
mod transcript;

pub use data::{ResponseStats, StatusData, UsageData};
pub use gather::gather_from_json;
pub use render::render;
