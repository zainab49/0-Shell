//! `help` — document the built-in commands.

use crate::commands::{CmdResult, Io};

/// One row of the help table: command name and a short description.
const ENTRIES: &[(&str, &str)] = &[
    ("echo [-n] [text...]", "print text (use -n to omit the trailing newline)"),
    ("cd [dir]", "change directory (no argument goes to $HOME)"),
    ("pwd", "print the working directory"),
    ("ls [-l] [-a] [-F] [path...]", "list directory contents"),
    ("cat [file...]", "print files, or standard input when given none"),
    ("cp src... dest", "copy files"),
    ("rm [-r] path...", "remove files (and directories with -r)"),
    ("mv src... dest", "move or rename files"),
    ("mkdir [-p] dir...", "create directories"),
    ("help", "show this help text"),
    ("exit [code]", "quit the shell"),
];

/// Prints an overview of the built-in commands and supported features.
pub fn run(_args: &[String], io: &mut Io) -> CmdResult {
    writeln!(io.out, "0-shell built-in commands:").map_err(stringify)?;
    for (name, description) in ENTRIES {
        writeln!(io.out, "  {name:<30} {description}").map_err(stringify)?;
    }
    writeln!(io.out).map_err(stringify)?;
    writeln!(
        io.out,
        "Features: ; command chaining, | pipes, < > >> redirection, $VAR expansion.\n\
         Interactive: command history (Up/Down), Tab completion, Ctrl+C to cancel, Ctrl+D to exit."
    )
    .map_err(stringify)?;
    Ok(())
}

fn stringify(e: std::io::Error) -> String {
    e.to_string()
}
