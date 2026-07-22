//! `cd` — change the shell's working directory.

use std::env;
use std::path::PathBuf;

use crate::commands::CmdResult;

/// Changes directory to the given path, or to `$HOME` when called with no
/// argument. Rejects extra operands, matching typical shell behaviour.
pub fn run(args: &[String]) -> CmdResult {
    if args.len() > 1 {
        return Err("too many arguments".to_string());
    }

    let target: PathBuf = match args.first() {
        Some(path) => PathBuf::from(path),
        None => home_dir()?,
    };

    env::set_current_dir(&target)
        .map_err(|e| format!("{}: {}", target.display(), e))
}

/// Resolves the user's home directory from the environment.
fn home_dir() -> Result<PathBuf, String> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME not set".to_string())
}
