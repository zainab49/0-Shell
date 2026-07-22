//! The interactive read-eval-print loop.

use std::env;
use std::path::PathBuf;

use crate::color;
use crate::executor::{self, Flow};
use crate::history::History;
use crate::line_editor::{self, Line};
use crate::parser;
use crate::terminal;

/// Runs the shell loop until EOF (Ctrl+D) or an `exit` command.
///
/// Returns the process exit code.
pub fn run() -> i32 {
    // Never let Ctrl+C kill the shell itself.
    terminal::ignore_sigint();

    let interactive = terminal::is_tty(terminal::STDIN_FILENO);
    let mut history = History::load();

    loop {
        let (prompt, width) = build_prompt(interactive);

        let line = match line_editor::read_line(&prompt, width, history.entries()) {
            Ok(line) => line,
            Err(err) => {
                eprintln!("0-shell: read error: {err}");
                history.save();
                return 1;
            }
        };

        let input = match line {
            Line::Input(text) => text,
            Line::Interrupted => continue, // Ctrl+C: fresh prompt
            Line::Eof => break,
        };

        if input.trim().is_empty() {
            continue;
        }
        history.add(&input);

        match parser::parse(&input) {
            Ok(pipelines) => {
                if let Flow::Exit(code) = executor::run_list(pipelines) {
                    history.save();
                    return code;
                }
            }
            Err(message) => report_syntax_error(&message),
        }
    }

    history.save();
    0
}

/// Builds the prompt string and its visible width (in columns).
///
/// Interactive sessions get a colourised, `~`-collapsed working directory;
/// non-interactive input keeps the plain `$ ` prompt.
fn build_prompt(interactive: bool) -> (String, usize) {
    if !interactive {
        return ("$ ".to_string(), 2);
    }

    let path = current_dir_display();
    let symbol = " $ ";
    let width = path.chars().count() + symbol.chars().count();

    let color_on = terminal::is_tty(terminal::STDOUT_FILENO);
    let prompt = format!("{}{}", color::paint(color_on, color::CYAN, &path), symbol);
    (prompt, width)
}

/// Returns the current directory with `$HOME` collapsed to `~`.
fn current_dir_display() -> String {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("?"));

    if let Some(home) = env::var_os("HOME") {
        let home = PathBuf::from(home);
        if let Ok(rest) = cwd.strip_prefix(&home) {
            return if rest.as_os_str().is_empty() {
                "~".to_string()
            } else {
                format!("~/{}", rest.display())
            };
        }
    }
    cwd.display().to_string()
}

/// Reports a parser/syntax error to standard error, in red when appropriate.
fn report_syntax_error(message: &str) {
    let colored = terminal::is_tty(terminal::STDERR_FILENO);
    let text = format!("0-shell: {message}");
    eprintln!("{}", color::paint(colored, color::BOLD_RED, &text));
}
