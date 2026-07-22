//! Built-in command implementations and shared helpers.

pub mod cat;
pub mod cd;
pub mod cp;
pub mod echo;
pub mod ls;
pub mod mkdir;
pub mod mv;
pub mod pwd;
pub mod rm;

mod util;

/// The outcome of a single built-in command as far as the shell loop cares.
pub enum CommandResult {
    /// Keep prompting for more input.
    Continue,
    /// Terminate the shell with the given exit code.
    Exit(i32),
}

/// Result type shared by all built-ins. The `String` is a human-readable
/// error message, printed by the loop as `command: message`.
pub type CmdResult = Result<(), String>;
