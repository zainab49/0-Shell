//! `echo` — write arguments to standard output.

use crate::commands::{CmdResult, Io};

/// Prints the arguments separated by single spaces, followed by a newline.
///
/// Supports the common `-n` flag, which suppresses the trailing newline.
pub fn run(args: &[String], io: &mut Io) -> CmdResult {
    let (suppress_newline, rest) = match args.first() {
        Some(flag) if flag == "-n" => (true, &args[1..]),
        _ => (false, args),
    };

    let line = rest.join(" ");
    let result = if suppress_newline {
        write!(io.out, "{line}")
    } else {
        writeln!(io.out, "{line}")
    };
    result.map_err(|e| e.to_string())
}
