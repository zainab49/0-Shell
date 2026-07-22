//! Built-in command implementations and shared types.

use std::io::{Read, Write};

pub mod cat;
pub mod cd;
pub mod cp;
pub mod echo;
pub mod help;
pub mod ls;
pub mod mkdir;
pub mod mv;
pub mod pwd;
pub mod rm;

mod util;

/// The names of every built-in command, used for dispatch and tab-completion.
pub const BUILTINS: &[&str] = &[
    "echo", "cd", "pwd", "ls", "cat", "cp", "rm", "mv", "mkdir", "help", "exit",
];

/// Result type shared by all built-ins. The `String` is a human-readable
/// error message, printed by the executor as `command: message`.
pub type CmdResult = Result<(), String>;

/// The input/output environment a command runs in.
///
/// Commands write their normal output to `out` and, where relevant, read from
/// `input`. Both are trait objects so the executor can transparently point
/// them at the terminal, a file (redirection) or an in-memory buffer (pipes).
/// `color` reflects whether `out` is a terminal that should receive ANSI
/// colour codes.
pub struct Io<'a> {
    pub out: &'a mut dyn Write,
    pub input: &'a mut dyn Read,
    pub color: bool,
}
