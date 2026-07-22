//! `pwd` — print the current working directory.

use std::env;

use crate::commands::CmdResult;

/// Prints the absolute path of the current working directory.
pub fn run(_args: &[String]) -> CmdResult {
    let cwd = env::current_dir().map_err(|e| e.to_string())?;
    println!("{}", cwd.display());
    Ok(())
}
