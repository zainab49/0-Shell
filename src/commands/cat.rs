//! `cat` — concatenate files to standard output.

use std::fs::File;
use std::io::{self, Read, Write};

use crate::commands::CmdResult;

/// Writes the contents of each file argument to standard output. With no
/// arguments, copies standard input to standard output until EOF.
///
/// Errors from individual files are reported but do not stop processing of
/// the remaining files.
pub fn run(args: &[String]) -> CmdResult {
    let stdout = io::stdout();
    let mut out = stdout.lock();

    if args.is_empty() {
        let mut stdin = io::stdin();
        io::copy(&mut stdin, &mut out).map_err(|e| e.to_string())?;
        return Ok(());
    }

    // Per-file errors are reported inline so that a bad path does not abort
    // the whole command; the loop keeps the shell alive regardless.
    for path in args {
        let result = File::open(path).and_then(|mut file| copy_to(&mut file, &mut out));
        if let Err(e) = result {
            eprintln!("cat: {path}: {e}");
        }
    }

    Ok(())
}

/// Streams a reader into a writer in fixed-size chunks to bound memory use.
fn copy_to<R: Read, W: Write>(reader: &mut R, writer: &mut W) -> io::Result<()> {
    let mut buffer = [0u8; 8192];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            return Ok(());
        }
        writer.write_all(&buffer[..n])?;
    }
}
