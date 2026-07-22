//! 0-shell: a minimalist Unix-like shell written in Rust.
//!
//! The shell reads a line at a time, tokenises it (honouring single and
//! double quotes), and dispatches to a built-in command implemented purely
//! with Rust's standard library — no external binaries are ever spawned.

mod commands;
mod parser;
mod shell;

fn main() {
    // Run the interactive loop. The process exit code mirrors the shell's
    // final state so it composes well with other tools.
    let exit_code = shell::run();
    std::process::exit(exit_code);
}
