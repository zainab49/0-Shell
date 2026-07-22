//! `cp` — copy files.

use std::fs;
use std::path::{Path, PathBuf};

use crate::commands::util;
use crate::commands::CmdResult;

/// Copies one or more source files to a destination.
///
/// * `cp SRC DST` copies `SRC` to `DST` (a file or into an existing dir).
/// * `cp SRC... DIR` copies every source into the existing directory `DIR`.
///
/// Directories are rejected, matching `cp` without `-r`.
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
        if let Err(e) = copy_one(src, &target) {
            eprintln!("cp: cannot copy '{source}': {e}");
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

/// Copies a single regular file, rejecting directories.
fn copy_one(src: &Path, target: &Path) -> std::io::Result<()> {
    let metadata = fs::symlink_metadata(src)?;
    if metadata.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "omitting directory (recursive copy not supported)",
        ));
    }
    fs::copy(src, target)?;
    Ok(())
}
