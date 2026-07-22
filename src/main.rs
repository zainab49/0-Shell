//! 0-shell: a minimalist Unix-like shell written in Rust.
//!
//! The shell reads a line at a time, parses it into pipelines (honouring
//! quotes, `$VAR` expansion, `;` chaining, `|` pipes and `< > >>`
//! redirection), and dispatches to built-in commands implemented purely with
//! Rust's standard library — no external binaries are ever spawned.
//!
//! Interactive sessions add a raw-mode line editor with command history and
//! Tab completion, a working-directory prompt, colourised output, and safe
//! Ctrl+C handling.

mod color;
mod commands;
mod executor;
mod history;
mod line_editor;
mod parser;
mod shell;
mod terminal;

fn main() {
    let exit_code = shell::run();
    std::process::exit(exit_code);
}
