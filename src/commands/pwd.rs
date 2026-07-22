//! `pwd` — print the current working directory.

use std::env;

use crate::commands::{CmdResult, Io};

/// Prints the absolute path of the current working directory.
pub fn run(_args: &[String], io: &mut Io) -> CmdResult {
    let cwd = env::current_dir().map_err(|e| e.to_string())?;
    writeln!(io.out, "{}", cwd.display()).map_err(|e| e.to_string())
}
