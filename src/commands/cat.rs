//! `cat` — concatenate files to standard output.

use std::fs::File;
use std::io;

use crate::commands::{CmdResult, Io};

/// Writes the contents of each file argument to the command's output. With no
/// arguments, copies the command's input to its output until EOF (this is what
/// makes `cat` useful on the receiving end of a pipe).
///
/// Errors from individual files are reported but do not stop processing of
/// the remaining files.
pub fn run(args: &[String], io: &mut Io) -> CmdResult {
    if args.is_empty() {
        io::copy(io.input, io.out).map_err(|e| e.to_string())?;
        return Ok(());
    }

    // Per-file errors are reported inline so a bad path does not abort the
    // whole command; the shell loop stays alive regardless.
    for path in args {
        let result = File::open(path).and_then(|mut file| io::copy(&mut file, io.out));
        if let Err(e) = result {
            eprintln!("cat: {path}: {e}");
        }
    }

    Ok(())
}
