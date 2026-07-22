# 0-shell

A minimalist Unix-like shell written in Rust. It runs core commands using only
the Rust standard library — no external binaries are ever spawned, and there
are no third-party crate dependencies.

## Features

- Interactive prompt (`$ `) that reads and executes one command at a time
- Graceful exit on `Ctrl+D` (EOF) and via the `exit` command
- Quote-aware argument parsing (`'single'` and `"double"` quotes)
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
  | `exit`  | quit the shell |

Unknown commands print `Command '<name>' not found`.

## Running with Docker (no Rust needed)

Your machine does not need Rust installed — everything compiles inside the
image.

```sh
# Build the image (first build downloads the Rust toolchain)
docker build -t zero-shell .

# Run the shell interactively
docker run --rm -it zero-shell
```

You will get a `$ ` prompt. Try:

```
$ pwd
$ mkdir demo
$ cd demo
$ echo "Hello There"
$ ls -la
$ exit
```

To experiment on real files, mount a folder into the container:

```sh
docker run --rm -it -v "$PWD:/work" -w /work zero-shell
```

## Project layout

```
src/
  main.rs              entry point
  shell.rs             read-eval-print loop and command dispatch
  parser.rs            quote-aware tokeniser (with unit tests)
  commands/
    mod.rs             shared types
    util.rs            shared helpers
    echo.rs cd.rs pwd.rs cat.rs cp.rs rm.rs mv.rs mkdir.rs
    ls.rs              long/-a/-F listing, permission + time formatting (tested)
```

## Building and testing locally (if you have Rust)

```sh
cargo build --release   # binary at target/release/0-shell
cargo test              # run the unit tests
cargo run               # start the shell
```
