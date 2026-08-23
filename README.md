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

## Running the shell

### With Rust installed

```sh
cargo run --release      # build and start the shell
cargo build --release    # just build; binary at target/release/0-shell
cargo test               # run the unit tests
```

### With Docker (no Rust needed)

Everything compiles inside the image, so the host needs nothing but Docker.

```sh
# Build the image (first build downloads the Rust toolchain)
docker build -t zero-shell .

# Run the shell interactively (-it is required for the line editor)
docker run --rm -it zero-shell
```

To experiment on real files, mount a folder into the container:

```sh
docker run --rm -it -v "$PWD:/work" -w /work zero-shell
```

Either way you get a prompt showing the working directory, like `~ $`.

### Feeding it a script instead of typing

The shell reads non-terminal input line by line, so you can pipe a script in:

```sh
printf 'pwd\nls -la\nexit\n' | ./target/release/0-shell
./target/release/0-shell < my-script.txt
```

In that mode the prompt degrades to a plain `$ ` and the line editor
(history, Tab, arrows) is skipped — handy for testing and CI.

## Command reference — copy these and try them

Every block below can be pasted straight into the running shell. A safe place
to play is a scratch directory:

```
mkdir -p /tmp/shell-demo ; cd /tmp/shell-demo ; pwd
```

On Windows use a path that exists, e.g. `mkdir demo ; cd demo`.

### `echo` — print text

```
echo hello world
echo -n "no trailing newline"
echo
echo "a    b"
```

`echo` joins its arguments with a single space and adds a newline. `-n`
suppresses that newline, which is why the next prompt appears on the same
line. The bare `echo` prints an empty line. Quoting `"a    b"` keeps the inner
spaces, since without quotes the parser treats a run of whitespace as nothing
more than an argument separator.

### `pwd` and `cd` — move around

```
pwd
mkdir -p one/two/three
cd one/two
pwd
cd ..
pwd
cd
pwd
```

`pwd` prints the absolute working directory. `cd` accepts a relative or
absolute path, `..` to go up, and no argument at all to jump to `$HOME`.
Passing two or more paths is an error (`cd: too many arguments`).

> If `$HOME` is not set — which is the case when you launch the shell from
> PowerShell or `cmd` — bare `cd` reports `cd: HOME not set`, and the prompt
> shows the full path instead of collapsing it to `~`. Set `HOME` before
> starting, or run under Git Bash / Docker where it is already set.

### `ls` — list directory contents

```
ls
ls -a
ls -F
ls -l
ls -la
ls -lF one
ls one two.txt
ls nosuchfile
```

- plain `ls` lists the current directory, sorted the way GNU `ls` sorts it
  (case-insensitively, ignoring a leading dot), and hides dotfiles
- `-a` adds hidden entries plus the synthetic `.` and `..`
- `-F` appends a type indicator: `/` directory, `@` symlink, `*` executable,
  `|` FIFO, `=` socket
- `-l` is the long format — permissions, link count, owner, group, size,
  modification time, name — with a `total` line counting 1 KiB blocks
- flags combine freely in one word (`-la`, `-lF`, `-alF`) or separately
  (`-l -a`)
- multiple operands print file operands first, then each directory under a
  `name:` header
- an unreadable path reports `ls: cannot access '...'` and the other operands
  still list; an unknown flag reports `ls: invalid option -- 'Z'`

After `echo first > f.txt ; mkdir one`, a `-l` listing looks like this:

```
total 0
-rw-r--r-- 1 zainab zainab 6 Aug 23 15:33 f.txt
drwxr-xr-x 1 zainab zainab 0 Aug 23 15:33 one
```

Columns are padded to their widest value, as GNU `ls` does. Times are UTC, and
anything older than roughly six months shows the year in place of `HH:MM`,
matching coreutils. The `total` counts allocated blocks, so it stays 0 for
small files off Unix, where block counts are derived from the logical size
rather than read from `stat`.

### `cat` — print files

```
echo first > f.txt
cat f.txt
cat f.txt f.txt
cat f.txt nosuch.txt
```

`cat` writes each file to the output in order. A missing file prints
`cat: nosuch.txt: ...` but does **not** abort the command — the remaining
files are still printed.

With no arguments `cat` copies its input to its output, which is what makes it
work on the receiving end of a pipe (`... | cat`). Typed at an interactive
prompt with no arguments, it reads what you type until you press `Ctrl+D`.

### `mkdir` — create directories

```
mkdir alpha
mkdir alpha
mkdir -p deep/nested/path
mkdir -p deep/nested/path
mkdir
```

The second `mkdir alpha` fails, because the directory already exists. `-p`
creates every missing parent **and** treats an existing target as success, so
repeating it is harmless. With no operand you get `mkdir: missing operand`.

### `cp` — copy files

```
echo data > src.txt
cp src.txt copy.txt
mkdir bucket
cp src.txt bucket
cp src.txt copy.txt bucket
cp src.txt
cp bucket other.txt
ls bucket
```

- `cp SRC DST` copies to `DST`, or into `DST` when that is an existing
  directory
- `cp SRC... DIR` copies several sources into an existing directory
- several sources with a non-directory destination is an error:
  `target '...' is not a directory`
- one argument gives `cp: missing file operand`
- there is no `-r`, so a directory source reports
  `omitting directory (recursive copy not supported)`

### `mv` — move and rename

```
mv copy.txt renamed.txt
mv renamed.txt bucket
mv src.txt bucket/inner.txt
ls bucket
```

Same operand shapes as `cp`: `mv SRC DST` renames (or moves into `DST` if it
is an existing directory), and `mv SRC... DIR` moves several sources into a
directory. Unlike `cp`, `mv` handles directories fine, since it is a rename.

### `rm` — remove files and directories

```
rm bucket/inner.txt
rm bucket
rm -r bucket
rm -rf deep
rm nosuchfile
rm
rm -q file
```

`rm` deletes files. A directory needs `-r` (or `-R`), otherwise you get
`rm: cannot remove 'bucket': is a directory`. Flags may be clustered as `-rf`;
`f` is accepted for familiarity. A symlink is removed as the link, not
followed to its target. Any other letter is rejected with
`invalid option '-q'`, and no operands gives `rm: missing operand`.

### Quoting and escaping

```
echo 'single quotes are literal: $HOME  |  ;  >'
echo "double quotes expand: $HOME"
echo "escaped \$HOME stays a dollar sign"
echo "she said \"hi\""
echo one\ word
echo a''b"c"d
echo ""
```

- **single quotes** are fully literal — no variable expansion, and `;`, `|`
  and `>` lose their special meaning
- **double quotes** keep spaces but still expand `$VAR`; inside them a
  backslash escapes only `"`, `\` and `$` (any other backslash stays literal)
- **backslash** outside quotes escapes the very next character, so
  `one\ word` is a single argument
- adjacent quoted and bare segments join into one word, so `a''b"c"d` prints
  `abcd`
- `""` is a real, empty argument — `echo ""` prints a blank line

An unterminated quote is caught before anything runs:

```
echo "oops
```

reports ``0-shell: unexpected EOF while looking for matching `"` ``.

### Environment variables

```
echo $HOME
echo ${HOME}
echo "path is $PATH"
echo "unset --> ${NOPE} <--"
echo prefix${HOME}suffix
echo 100$
echo $1abc
```

`$NAME` and `${NAME}` are read from the environment wherever they appear
outside single quotes. A name is `[A-Za-z_][A-Za-z0-9_]*`; the braced form
lets you butt a variable straight against following text. An **unset variable
expands to nothing** (no error). A lone `$`, or a `$` followed by something
that cannot start a name, is left as a literal `$`.

Note that the shell has no `export` built-in — it reads the environment it was
launched with, it does not modify it.

### Chaining with `;`

```
pwd ; echo second ; echo third
mkdir chain ; cd chain ; pwd ; cd ..
echo before ; badcmd ; echo after
ls ;
```

Statements run left to right, each one independent. A failure does not stop
the rest — `echo after` still runs. A trailing or doubled `;` is ignored.

### Pipes with `|`

```
echo hello | cat
cat f.txt | cat | cat
ls -la | cat
```

Each stage's output becomes the next stage's input. Because every command is a
built-in running synchronously, stages execute left to right with the data
buffered in memory rather than truly concurrently — for these commands the
visible result is the same.

Colour is deliberately switched off for anything that is not the final stage
writing to a real terminal, so `ls | cat` gives you clean, unescaped text.

An empty stage is a syntax error:

```
| cat
echo x | | cat
```

Both report ``0-shell: syntax error near unexpected token `|` ``.

> One gotcha: a pipeline whose **first** stage is a bare `cat` reads the real
> standard input. Interactively that means `cat | cat` waits for you to type
> and press `Ctrl+D`. If you are piping a script into the shell, that `cat`
> will swallow the rest of your script.

### Redirection: `>`, `>>`, `<`

```
echo first > out.txt
cat out.txt
echo second > out.txt
cat out.txt
echo third >> out.txt
cat out.txt
cat < out.txt
ls -la > listing.txt ; cat listing.txt
echo x >
```

- `>` writes to the file, **truncating** it — after the second `echo`,
  `out.txt` holds only `second`
- `>>` appends, so the file then holds `second` and `third`
- `<` feeds a file in as the command's input
- a redirection with no filename is a syntax error:
  ``syntax error near unexpected token after `>` ``
- an unwritable target is reported as `echo: /bad/path: ...` and the shell
  keeps going

Redirection binds per command inside a pipeline, and an explicit `>` on a
non-final stage wins over the pipe.

### Combining everything

```
mkdir -p project/src ; cd project
echo "fn main() {}" > src/main.rs
echo "# a project for $USER" > README.md
ls -laF ; echo "---" ; cat src/main.rs
cat README.md >> src/main.rs ; cat src/main.rs
cat src/main.rs | cat > combined.txt ; ls -l combined.txt
cd .. ; rm -r project ; ls
```

This exercises `-p`, quoting, variable expansion, chaining, both redirection
forms, a pipe into a redirect, and recursive removal in one pass.

### Errors and edge cases worth seeing

```
badcommand
ls -Z
cd nosuchdir
cd a b
cat missing.txt
rm
cp only-one-arg
echo "unclosed
```

| Input | Result |
|-------|--------|
| `badcommand` | `Command 'badcommand' not found` |
| `ls -Z` | `ls: invalid option -- 'Z'` |
| `cd nosuchdir` | `cd: nosuchdir: <os error>` |
| `cd a b` | `cd: too many arguments` |
| `cat missing.txt` | `cat: missing.txt: <os error>` |
| `rm` | `rm: missing operand` |
| `cp only-one-arg` | `cp: missing file operand` |
| `echo "unclosed` | ``0-shell: unexpected EOF while looking for matching `"` `` |

Nothing here kills the shell — every error prints a diagnostic (in red on a
terminal) and returns you to the prompt.

### Interactive keys — try these by hand

These cannot be pasted; type them.

| Key | Effect |
|-----|--------|
| `Up` / `Down` | walk through command history |
| `Tab` | complete a built-in name (first word) or a path (later words) |
| `Ctrl+C` | abandon the current line and get a fresh prompt — never exits |
| `Ctrl+D` | exit the shell (only on an empty line) |
| `Left` / `Right` | move the cursor |
| `Home` / `Ctrl+A` | jump to start of line |
| `End` / `Ctrl+E` | jump to end of line |
| `Backspace` / `Delete` | delete before / under the cursor |

To see history persist, run some commands, `exit`, restart the shell and press
`Up`. It is stored in `~/.0-shell_history`.

For Tab completion, type `mk` then Tab (completes to `mkdir`), or `cat R` then
Tab in this repository (completes to `README.md`).

### `help` and `exit`

```
help
exit
exit 3
```

`help` prints the built-in table and a feature summary. `exit` quits with
status 0; `exit 3` quits with status 3 — check it with `echo $?` in your outer
shell. A non-numeric argument is ignored and treated as 0. `Ctrl+D` is
equivalent to a bare `exit`.

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

## Portability notes

- Globbing (`*`, `?`) is intentionally **not** implemented — `ls *.txt` looks
  for a file literally named `*.txt`.
- The shell never spawns external programs, so only the built-ins above exist.
  There is no `grep`, `wc`, `git`, or anything else.
- `ls -l` reads `/etc/passwd` and `/etc/group` for owner and group names. Off
  Unix there is no such database, so those columns fall back to the value of
  `$USERNAME`, and the mode string is synthesised from the file type and the
  read-only flag (the same approach WSL and Git for Windows take).
- On non-terminal input (a pipe or a redirected file) the raw-mode line editor
  is skipped in favour of plain line reading, so scripted use works everywhere.
