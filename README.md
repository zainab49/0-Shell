# 0-shell

A minimalist Unix-like shell written in Rust. It runs core commands using only
the Rust standard library — no external binaries are ever spawned, and there
are no third-party crate dependencies.

## Core features

- Interactive prompt that reads and executes one command at a time
- Graceful exit on `Ctrl+D` (EOF) and via the `exit` command
- Quote-aware parsing (`'single'` and `"double"` quotes, `\` escaping)
- Built-in commands:

  | Command | Notes |
  |---------|-------|
  | `echo`  | supports `-n` |
  | `cd`    | no argument goes to `$HOME` |
  | `pwd`   | |
  | `ls`    | supports `-l`, `-a`, `-F` (and combinations like `-la`) |
  | `cat`   | reads files, or standard input when given none |
  | `cp`    | copies files; `cp SRC... DIR` into a directory |
  | `rm`    | supports `-r` for directories |
  | `mv`    | rename or move into a directory |
  | `mkdir` | supports `-p` |
  | `help`  | documents the built-ins |
  | `exit`  | quit the shell (optional exit code) |

Unknown commands print `Command '<name>' not found`.

## Bonus features

All of the project's bonus features are implemented — still with zero external
crates (the terminal handling uses small hand-written bindings to the C library
Rust already links against):

- **Ctrl+C (SIGINT)** never crashes the shell — it cancels the current line
- **Command history** — navigate previous commands with the Up/Down arrows;
  persisted to `~/.0-shell_history`
- **Tab auto-completion** — completes built-in names (command position) and
  file/directory paths (argument position)
- **Working-directory prompt** — shows the current directory with `$HOME`
  collapsed to `~`
- **Colourised output** — directories, executables, symlinks, the prompt and
  error messages (only when writing to a terminal)
- **Command chaining** with `;`
- **Pipes** with `|`
- **I/O redirection**: `>`, `>>` and `<`
- **Environment variables**: `$VAR` and `${VAR}` (expanded outside single
  quotes)

Other line-editing keys: Left/Right to move, Home/End (or Ctrl+A/Ctrl+E),
Backspace and Delete.

## Running with Docker (no Rust needed)

Your machine does not need Rust installed — everything compiles inside the
image.

```sh
# Build the image (first build downloads the Rust toolchain)
docker build -t zero-shell .

# Run the shell interactively (-it is required for the line editor)
docker run --rm -it zero-shell
```

You will get a prompt like `~ $`. Try:

```
$ pwd
$ mkdir demo
$ cd demo
$ echo "Hello There" > greeting.txt
$ cat greeting.txt | cat
$ echo "home is $HOME"
$ ls -la ; help
$ exit
```

To experiment on real files, mount a folder into the container:

```sh
docker run --rm -it -v "$PWD:/work" -w /work zero-shell
```

## Project layout

```
src/
  main.rs              entry point and module wiring
  shell.rs             read-eval-print loop and prompt
  parser.rs            lexer + parser: quotes, $VAR, ; | < > >>  (tested)
  executor.rs          runs pipelines, redirection, built-in dispatch
  line_editor.rs       raw-mode editor: history, completion, key handling
  history.rs           command-history storage and persistence
  terminal.rs          hand-written termios/isatty/signal bindings
  color.rs             ANSI colour helpers
  commands/
    mod.rs             shared Io type and built-in registry
    util.rs            shared helpers
    echo.rs cd.rs pwd.rs cat.rs cp.rs rm.rs mv.rs mkdir.rs help.rs
    ls.rs              long/-a/-F listing, colours, permission + time (tested)
```

## Building and testing locally (if you have Rust)

```sh
cargo build --release   # binary at target/release/0-shell
cargo test              # run the unit tests
cargo run               # start the shell
```

The interactive line editor (history, completion, raw-mode keys) targets
Unix-like systems. On non-terminal input (pipes, redirected files) the shell
falls back to plain line reading, so scripted use works everywhere.
