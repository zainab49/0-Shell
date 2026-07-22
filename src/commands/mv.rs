//! `mv` — move (rename) files and directories.

use std::fs;
use std::path::{Path, PathBuf};

use crate::commands::util;
use crate::commands::CmdResult;

/// Moves one or more sources to a destination.
///
/// * `mv SRC DST` renames `SRC` to `DST`, or moves it into `DST` if that is
///   an existing directory.
/// * `mv SRC... DIR` moves every source into the existing directory `DIR`.
pub fn run(args: &[String]) -> CmdResult {
    if args.len() < 2 {
        return Err("missing file operand".to_string());
    }

    let (sources, dest) = args.split_at(args.len() - 1);
    let dest = Path::new(&dest[0]);
    let dest_is_dir = dest.is_dir();

    if sources.len() > 1 && !dest_is_dir {
        return Err(format!("target '{}' is not a directory", dest.display()));
    }

    for source in sources {
        let src = Path::new(source);
        let target = resolve_target(src, dest, dest_is_dir);
        if let Err(e) = fs::rename(src, &target) {
            eprintln!("mv: cannot move '{source}': {e}");
        }
    }

    Ok(())
}

/// Computes the concrete destination path for a source: inside `dest` when it
/// is a directory, otherwise `dest` itself.
fn resolve_target(src: &Path, dest: &Path, dest_is_dir: bool) -> PathBuf {
    if dest_is_dir {
        match util::file_name(src) {
            Some(name) => dest.join(name),
            None => dest.to_path_buf(),
        }
    } else {
        dest.to_path_buf()
    }
}
