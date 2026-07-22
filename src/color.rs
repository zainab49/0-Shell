//! ANSI colour helpers.
//!
//! All helpers take a `enabled` flag so callers can transparently disable
//! colouring when output is not a terminal (for example when redirected to a
//! file or piped into another command).

pub const RESET: &str = "\x1b[0m";
pub const BOLD_BLUE: &str = "\x1b[1;34m"; // directories
pub const BOLD_GREEN: &str = "\x1b[1;32m"; // executables
pub const BOLD_CYAN: &str = "\x1b[1;36m"; // symbolic links
pub const BOLD_RED: &str = "\x1b[1;31m"; // errors
pub const CYAN: &str = "\x1b[36m"; // prompt path

/// Wraps `text` in the given ANSI code when `enabled`, otherwise returns it
/// unchanged.
pub fn paint(enabled: bool, code: &str, text: &str) -> String {
    if enabled {
        format!("{code}{text}{RESET}")
    } else {
        text.to_string()
    }
}
