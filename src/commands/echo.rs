//! `echo` — write arguments to standard output.

use crate::commands::CmdResult;

/// Prints the arguments separated by single spaces, followed by a newline.
///
/// Supports the common `-n` flag, which suppresses the trailing newline.
pub fn run(args: &[String]) -> CmdResult {
    let (suppress_newline, rest) = match args.first() {
        Some(flag) if flag == "-n" => (true, &args[1..]),
        _ => (false, args),
    };

    let line = rest.join(" ");
    if suppress_newline {
        print!("{line}");
    } else {
        println!("{line}");
    }
    Ok(())
}
