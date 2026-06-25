//! ANSI palette + nerd-font icons + bar geometry, ported from statusline.nu.

// Foreground colors (Nushell `ansi` names noted alongside).
pub(crate) const RESET: &str = "\x1b[0m";
pub(crate) const BOLD: &str = "\x1b[1m"; // attr_bold
pub(crate) const DIM: &str = "\x1b[2m"; // attr_dimmed
pub(crate) const BLUE: &str = "\x1b[94m"; // light_blue
pub(crate) const GREEN: &str = "\x1b[92m"; // light_green
pub(crate) const YELLOW: &str = "\x1b[93m"; // light_yellow
pub(crate) const RED: &str = "\x1b[91m"; // light_red
pub(crate) const CYAN: &str = "\x1b[96m"; // light_cyan
pub(crate) const MAGENTA: &str = "\x1b[95m"; // light_magenta
pub(crate) const ORANGE: &str = "\x1b[33m"; // ansi yellow, used as orange
pub(crate) const WHITE: &str = "\x1b[37m";

// 256-color bar backgrounds: tinted fill + dark track.
pub(crate) const BG_TRACK: &str = "\x1b[48;5;236m";
pub(crate) const BG_GREEN: &str = "\x1b[48;5;22m";
pub(crate) const BG_ORANGE: &str = "\x1b[48;5;94m";
pub(crate) const BG_RED: &str = "\x1b[48;5;52m";

// Nerd-font icons.
pub(crate) const ICON_MODEL: &str = "󰚩";
pub(crate) const ICON_FOLDER: &str = "󰉋";
pub(crate) const ICON_CONTEXT: &str = "󰘚";
pub(crate) const ICON_TOKENS: &str = "󰦨";
pub(crate) const ICON_TIME: &str = "󱑆";
pub(crate) const ICON_USAGE: &str = "󰄪";
pub(crate) const ICON_GIT: &str = "󰘬";
pub(crate) const ICON_RESET: &str = "󱫤";

/// Progress-bar width in cells.
pub(crate) const BAR_WIDTH: usize = 20;

/// Braille fill levels, bottom→top: blank … full (8 vertical sub-steps).
pub(crate) const BRAILLE: [&str; 9] = ["⠀", "⡀", "⡄", "⡆", "⡇", "⣇", "⣧", "⣷", "⣿"];
