//! Executes parsed pipelines: dispatches built-ins, wires up redirections and
//! connects piped commands through in-memory buffers.

use std::fs::{File, OpenOptions};
use std::io::{self, Cursor, Read, Write};

use crate::color;
use crate::commands::{self, Io};
use crate::parser::{Command, Pipeline};
use crate::terminal;

/// What should happen to the shell after running a statement list.
pub enum Flow {
    /// Keep looping.
    Continue,
    /// Exit the shell with the given status code.
    Exit(i32),
}

/// Runs every pipeline in order (the `;` separated statements).
pub fn run_list(pipelines: Vec<Pipeline>) -> Flow {
    for pipeline in pipelines {
        if let Flow::Exit(code) = run_pipeline(pipeline) {
            return Flow::Exit(code);
        }
    }
    Flow::Continue
}

/// Runs a single pipeline. Commands are executed left to right; each stage's
/// output is buffered and handed to the next stage's input. Because every
/// command is a synchronous built-in, sequential execution is sufficient.
fn run_pipeline(pipeline: Pipeline) -> Flow {
    let stdout_is_tty = terminal::is_tty(terminal::STDOUT_FILENO);
    let count = pipeline.commands.len();
    let mut piped_input: Vec<u8> = Vec::new();

    for (index, command) in pipeline.commands.iter().enumerate() {
        let is_first = index == 0;
        let is_last = index == count - 1;

        // Build the input reader for this stage.
        let mut input: Box<dyn Read> = match &command.stdin {
            Some(path) => match File::open(path) {
                Ok(file) => Box::new(file),
                Err(e) => {
                    report_error(name_of(command), &format!("{path}: {e}"));
                    continue;
                }
            },
            None if !is_first => Box::new(Cursor::new(std::mem::take(&mut piped_input))),
            None => Box::new(io::stdin()),
        };

        // Build the output writer for this stage and remember whether it is a
        // real terminal (so colours are only emitted there).
        let redirect = command.stdout.clone();
        let feeds_pipe = !is_last && redirect.is_none();

        let flow = if feeds_pipe {
            // Middle of a pipe: capture output for the next stage.
            let mut buffer: Vec<u8> = Vec::new();
            let flow = dispatch(command, &mut input, &mut buffer, false);
            piped_input = buffer;
            flow
        } else if let Some((path, append)) = redirect {
            match open_output(&path, append) {
                Ok(mut file) => {
                    let flow = dispatch(command, &mut input, &mut file, false);
                    let _ = file.flush();
                    flow
                }
                Err(e) => {
                    report_error(name_of(command), &format!("{path}: {e}"));
                    Flow::Continue
                }
            }
        } else {
            // Final stage writing to the terminal.
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            let flow = dispatch(command, &mut input, &mut handle, stdout_is_tty);
            let _ = handle.flush();
            flow
        };

        if let Flow::Exit(code) = flow {
            return Flow::Exit(code);
        }
    }

    Flow::Continue
}

/// Opens a redirection target, truncating or appending as requested.
fn open_output(path: &str, append: bool) -> io::Result<File> {
    if append {
        OpenOptions::new().create(true).append(true).open(path)
    } else {
        File::create(path)
    }
}

/// Dispatches a single command to its built-in implementation, wiring up the
/// supplied input/output. Returns the resulting control flow.
fn dispatch(command: &Command, input: &mut dyn Read, out: &mut dyn Write, color: bool) -> Flow {
    let name = command.argv[0].as_str();
    let args = &command.argv[1..];

    // `exit` is handled here because it affects the loop rather than producing
    // output like the other built-ins.
    if name == "exit" {
        let code = args
            .first()
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);
        return Flow::Exit(code);
    }

    let mut io = Io { out, input, color };

    let result = match name {
        "echo" => commands::echo::run(args, &mut io),
        "cd" => commands::cd::run(args, &mut io),
        "pwd" => commands::pwd::run(args, &mut io),
        "ls" => commands::ls::run(args, &mut io),
        "cat" => commands::cat::run(args, &mut io),
        "cp" => commands::cp::run(args, &mut io),
        "rm" => commands::rm::run(args, &mut io),
        "mv" => commands::mv::run(args, &mut io),
        "mkdir" => commands::mkdir::run(args, &mut io),
        "help" => commands::help::run(args, &mut io),
        other => {
            // The unknown-command message goes to the command's output stream
            // so it appears exactly where the spec's examples show it.
            let _ = writeln!(io.out, "Command '{other}' not found");
            return Flow::Continue;
        }
    };

    if let Err(message) = result {
        report_error(name, &message);
    }
    Flow::Continue
}

/// The command name, used only for error messages.
fn name_of(command: &Command) -> &str {
    command.argv.first().map(String::as_str).unwrap_or("")
}

/// Prints a `name: message` diagnostic to standard error, in red when stderr
/// is a terminal.
fn report_error(name: &str, message: &str) {
    let colored = terminal::is_tty(terminal::STDERR_FILENO);
    let text = format!("{name}: {message}");
    eprintln!("{}", color::paint(colored, color::BOLD_RED, &text));
}
