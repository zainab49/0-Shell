//! `rm` — remove files and directories.

use std::fs;
use std::path::Path;

use crate::commands::{CmdResult, Io};

/// Removes each operand. Directories are only removed when `-r` (recursive)
/// is supplied, matching standard `rm` semantics.
pub fn run(args: &[String], _io: &mut Io) -> CmdResult {
    let mut recursive = false;
    let mut operands: Vec<&String> = Vec::new();

    // A leading run of flags is accepted (e.g. `rm -r`); everything else is a
    // path operand. Flags may be combined as `-rf`-style clusters.
    for arg in args {
        if arg.starts_with('-') && arg.len() > 1 && operands.is_empty() {
            if arg.chars().skip(1).all(|c| c == 'r' || c == 'R' || c == 'f') {
                if arg.contains('r') || arg.contains('R') {
                    recursive = true;
                }
            } else {
                return Err(format!("invalid option '{arg}'"));
            }
        } else {
            operands.push(arg);
        }
    }

    if operands.is_empty() {
        return Err("missing operand".to_string());
    }

    for target in operands {
        if let Err(e) = remove(Path::new(target), recursive) {
            eprintln!("rm: cannot remove '{target}': {e}");
        }
    }

    Ok(())
}

/// Removes a single path, choosing the correct removal call based on whether
/// the target is a directory and whether recursion was requested.
fn remove(path: &Path, recursive: bool) -> std::io::Result<()> {
    // `symlink_metadata` avoids following symlinks, so we remove the link
    // itself rather than its target.
    let metadata = fs::symlink_metadata(path)?;

    if metadata.is_dir() {
        if recursive {
            fs::remove_dir_all(path)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "is a directory",
            ))
        }
    } else {
        fs::remove_file(path)
    }
}
