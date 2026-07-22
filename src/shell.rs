//! The interactive read-eval-print loop.

use std::io::{self, BufRead, Write};

use crate::commands::{self, CommandResult};
use crate::parser;

const PROMPT: &str = "$ ";

/// Runs the shell loop until EOF (Ctrl+D) or an `exit` command.
///
/// Returns the process exit code.
pub fn run() -> i32 {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        // Print the prompt and make sure it appears before we block on input.
        if write!(stdout, "{PROMPT}").is_err() || stdout.flush().is_err() {
            return 1;
        }

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            // read_line returns 0 bytes only at end-of-file (Ctrl+D).
            Ok(0) => {
                // Emit a trailing newline so the caller's prompt starts cleanly.
                println!();
                return 0;
            }
            Ok(_) => {}
            Err(err) => {
                eprintln!("0-shell: read error: {err}");
                return 1;
            }
        }

        match parser::tokenize(&line) {
            Ok(tokens) => {
                if tokens.is_empty() {
                    continue;
                }
                if let CommandResult::Exit(code) = dispatch(&tokens) {
                    return code;
                }
            }
            Err(msg) => eprintln!("0-shell: {msg}"),
        }
    }
}

/// Routes a tokenised command line to the matching built-in.
fn dispatch(tokens: &[String]) -> CommandResult {
    let name = tokens[0].as_str();
    let args = &tokens[1..];

    let outcome = match name {
        "echo" => commands::echo::run(args),
        "cd" => commands::cd::run(args),
        "pwd" => commands::pwd::run(args),
        "ls" => commands::ls::run(args),
        "cat" => commands::cat::run(args),
        "cp" => commands::cp::run(args),
        "rm" => commands::rm::run(args),
        "mv" => commands::mv::run(args),
        "mkdir" => commands::mkdir::run(args),
        "exit" => return CommandResult::Exit(0),
        other => {
            println!("Command '{other}' not found");
            return CommandResult::Continue;
        }
    };

    // Every built-in reports failures through a uniform channel so the loop
    // stays alive and the user sees a consistent error style.
    if let Err(msg) = outcome {
        eprintln!("{name}: {msg}");
    }
    CommandResult::Continue
}
