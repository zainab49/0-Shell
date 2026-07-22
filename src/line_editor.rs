//! A small line editor for interactive use, built directly on raw-mode
//! terminal input. It provides cursor movement, command history (Up/Down) and
//! Tab completion, and treats Ctrl+C and Ctrl+D the way a shell should.
//!
//! Only single-line editing is supported, which keeps the rendering logic
//! simple and predictable.

use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

use crate::commands::BUILTINS;
use crate::terminal::RawMode;

/// The result of reading one line of input.
pub enum Line {
    /// A completed line (without its trailing newline).
    Input(String),
    /// The user pressed Ctrl+C; the current line was abandoned.
    Interrupted,
    /// End of input (Ctrl+D on an empty line).
    Eof,
}

/// Reads a single line from the terminal, showing `prompt` (whose visible
/// width is `prompt_width` columns, excluding any colour escapes).
///
/// Falls back to a plain buffered read if raw mode cannot be enabled.
pub fn read_line(prompt: &str, prompt_width: usize, history: &[String]) -> io::Result<Line> {
    let Some(_raw) = RawMode::enable() else {
        return fallback_read_line(prompt);
    };

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    let mut buf: Vec<char> = Vec::new();
    let mut cursor = 0usize;
    // `hist_pos == history.len()` means "editing a fresh line"; lower indices
    // point into the history. `stash` remembers the fresh line while browsing.
    let mut hist_pos = history.len();
    let mut stash = String::new();

    render(&mut out, prompt, prompt_width, &buf, cursor)?;

    loop {
        let Some(byte) = read_byte(&mut reader)? else {
            // Raw-mode EOF behaves like Ctrl+D.
            return finish_eof(&mut out, &buf);
        };

        match byte {
            b'\r' | b'\n' => {
                write!(out, "\r\n")?;
                out.flush()?;
                return Ok(Line::Input(buf.iter().collect()));
            }
            0x03 => {
                // Ctrl+C: abandon the line and let the shell reprompt.
                write!(out, "^C\r\n")?;
                out.flush()?;
                return Ok(Line::Interrupted);
            }
            0x04 => {
                // Ctrl+D: EOF only when the line is empty.
                if buf.is_empty() {
                    return finish_eof(&mut out, &buf);
                }
            }
            0x7f | 0x08 => {
                if cursor > 0 {
                    buf.remove(cursor - 1);
                    cursor -= 1;
                    render(&mut out, prompt, prompt_width, &buf, cursor)?;
                }
            }
            0x01 => {
                cursor = 0;
                render(&mut out, prompt, prompt_width, &buf, cursor)?;
            }
            0x05 => {
                cursor = buf.len();
                render(&mut out, prompt, prompt_width, &buf, cursor)?;
            }
            b'\t' => {
                complete(&mut out, prompt, prompt_width, &mut buf, &mut cursor)?;
            }
            0x1b => {
                handle_escape(
                    &mut reader,
                    &mut out,
                    prompt,
                    prompt_width,
                    &mut buf,
                    &mut cursor,
                    history,
                    &mut hist_pos,
                    &mut stash,
                )?;
            }
            b if b >= 0x20 => {
                if let Some(ch) = read_char(&mut reader, b)? {
                    buf.insert(cursor, ch);
                    cursor += 1;
                    render(&mut out, prompt, prompt_width, &buf, cursor)?;
                }
            }
            _ => {}
        }
    }
}

/// Emits the closing newline for an EOF and returns [`Line::Eof`].
fn finish_eof(out: &mut impl Write, buf: &[char]) -> io::Result<Line> {
    if buf.is_empty() {
        write!(out, "\r\n")?;
        out.flush()?;
    }
    Ok(Line::Eof)
}

/// Redraws the current line: clears it, prints the prompt and buffer, then
/// positions the cursor.
fn render(
    out: &mut impl Write,
    prompt: &str,
    prompt_width: usize,
    buf: &[char],
    cursor: usize,
) -> io::Result<()> {
    let content: String = buf.iter().collect();
    write!(out, "\r\x1b[K{prompt}{content}")?;
    // Move the cursor to the correct column: carriage-return to column 0, then
    // forward by prompt width plus the cursor offset.
    write!(out, "\r")?;
    let target = prompt_width + cursor;
    if target > 0 {
        write!(out, "\x1b[{target}C")?;
    }
    out.flush()
}

/// Reads one raw byte, returning `None` at end of input.
fn read_byte(reader: &mut impl Read) -> io::Result<Option<u8>> {
    let mut b = [0u8; 1];
    match reader.read(&mut b)? {
        0 => Ok(None),
        _ => Ok(Some(b[0])),
    }
}

/// Assembles a full (possibly multi-byte UTF-8) character from a leading byte.
fn read_char(reader: &mut impl Read, first: u8) -> io::Result<Option<char>> {
    let extra = match first {
        b if b < 0x80 => 0,
        b if b >> 5 == 0b110 => 1,
        b if b >> 4 == 0b1110 => 2,
        b if b >> 3 == 0b11110 => 3,
        _ => return Ok(None), // invalid leading byte; ignore
    };

    let mut bytes = vec![first];
    for _ in 0..extra {
        match read_byte(reader)? {
            Some(b) => bytes.push(b),
            None => return Ok(None),
        }
    }

    Ok(String::from_utf8(bytes).ok().and_then(|s| s.chars().next()))
}

/// Handles an escape sequence (arrow keys, Home/End, Delete).
#[allow(clippy::too_many_arguments)]
fn handle_escape(
    reader: &mut impl Read,
    out: &mut impl Write,
    prompt: &str,
    prompt_width: usize,
    buf: &mut Vec<char>,
    cursor: &mut usize,
    history: &[String],
    hist_pos: &mut usize,
    stash: &mut String,
) -> io::Result<()> {
    let Some(intro) = read_byte(reader)? else {
        return Ok(());
    };
    if intro != b'[' && intro != b'O' {
        return Ok(());
    }

    let Some(code) = read_byte(reader)? else {
        return Ok(());
    };

    match code {
        b'A' => history_prev(buf, cursor, history, hist_pos, stash),
        b'B' => history_next(buf, cursor, history, hist_pos, stash),
        b'C' => {
            if *cursor < buf.len() {
                *cursor += 1;
            }
        }
        b'D' => {
            if *cursor > 0 {
                *cursor -= 1;
            }
        }
        b'H' => *cursor = 0,
        b'F' => *cursor = buf.len(),
        b'0'..=b'9' => {
            // Extended sequences terminated by '~', e.g. Home(1), Delete(3),
            // End(4).
            let mut n = u32::from(code - b'0');
            loop {
                match read_byte(reader)? {
                    Some(b'~') | None => break,
                    Some(d @ b'0'..=b'9') => n = n * 10 + u32::from(d - b'0'),
                    Some(_) => break,
                }
            }
            match n {
                1 | 7 => *cursor = 0,
                4 | 8 => *cursor = buf.len(),
                3 => {
                    if *cursor < buf.len() {
                        buf.remove(*cursor);
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }

    render(out, prompt, prompt_width, buf, *cursor)
}

/// Replaces the edit buffer with the previous history entry, if any.
fn history_prev(
    buf: &mut Vec<char>,
    cursor: &mut usize,
    history: &[String],
    hist_pos: &mut usize,
    stash: &mut String,
) {
    if *hist_pos == 0 {
        return;
    }
    if *hist_pos == history.len() {
        *stash = buf.iter().collect();
    }
    *hist_pos -= 1;
    *buf = history[*hist_pos].chars().collect();
    *cursor = buf.len();
}

/// Moves forward through history, restoring the stashed line at the end.
fn history_next(
    buf: &mut Vec<char>,
    cursor: &mut usize,
    history: &[String],
    hist_pos: &mut usize,
    stash: &str,
) {
    if *hist_pos >= history.len() {
        return;
    }
    *hist_pos += 1;
    *buf = if *hist_pos == history.len() {
        stash.chars().collect()
    } else {
        history[*hist_pos].chars().collect()
    };
    *cursor = buf.len();
}

/// Attempts Tab completion of the word ending at the cursor.
fn complete(
    out: &mut impl Write,
    prompt: &str,
    prompt_width: usize,
    buf: &mut Vec<char>,
    cursor: &mut usize,
) -> io::Result<()> {
    // Identify the word under the cursor and whether it is in command position.
    let start = buf[..*cursor]
        .iter()
        .rposition(|c| c.is_whitespace())
        .map(|i| i + 1)
        .unwrap_or(0);
    let word: String = buf[start..*cursor].iter().collect();
    let is_command = buf[..start].iter().all(|c| c.is_whitespace());

    let mut candidates = if is_command {
        BUILTINS
            .iter()
            .filter(|name| name.starts_with(&word))
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    } else {
        path_candidates(&word)
    };
    candidates.sort();
    candidates.dedup();

    if candidates.is_empty() {
        return Ok(());
    }

    let (replacement, trailing, listing) = if candidates.len() == 1 {
        let only = candidates.into_iter().next().unwrap();
        let trailing = if only.ends_with('/') { "" } else { " " };
        (only, trailing, None)
    } else {
        let prefix = common_prefix(&candidates);
        (prefix, "", Some(candidates))
    };

    // Rewrite the word in the buffer.
    let mut new_word: Vec<char> = replacement.chars().collect();
    new_word.extend(trailing.chars());
    buf.splice(start..*cursor, new_word.iter().copied());
    *cursor = start + new_word.len();

    if let Some(list) = listing {
        write!(out, "\r\n")?;
        write!(out, "{}\r\n", format_columns(&list))?;
    }
    render(out, prompt, prompt_width, buf, *cursor)
}

/// Produces filesystem completion candidates for a partial path.
fn path_candidates(word: &str) -> Vec<String> {
    let (dir_part, prefix) = match word.rfind('/') {
        Some(i) => (&word[..=i], &word[i + 1..]),
        None => ("", word),
    };
    let dir = if dir_part.is_empty() { "." } else { dir_part };

    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with(prefix) {
                continue;
            }
            let mut full = format!("{dir_part}{name}");
            if Path::new(&format!("{dir}/{name}")).is_dir() {
                full.push('/');
            }
            out.push(full);
        }
    }
    out
}

/// Longest common prefix of a set of strings.
fn common_prefix(items: &[String]) -> String {
    let Some(first) = items.first() else {
        return String::new();
    };
    let mut end = first.len();
    for item in &items[1..] {
        end = first
            .char_indices()
            .zip(item.char_indices())
            .take_while(|((_, a), (_, b))| a == b)
            .count()
            .min(end);
    }
    first.chars().take(end).collect()
}

/// Lays candidate names out on one line, showing just their basenames.
fn format_columns(items: &[String]) -> String {
    items
        .iter()
        .map(|item| item.trim_end_matches('/').rsplit('/').next().unwrap_or(item))
        .collect::<Vec<_>>()
        .join("  ")
}

/// A minimal reader used when raw mode is unavailable (e.g. piped input).
fn fallback_read_line(prompt: &str) -> io::Result<Line> {
    let mut out = io::stdout();
    write!(out, "{prompt}")?;
    out.flush()?;

    let mut line = String::new();
    match io::stdin().read_line(&mut line)? {
        0 => Ok(Line::Eof),
        _ => {
            let trimmed = line.trim_end_matches(['\r', '\n']).to_string();
            Ok(Line::Input(trimmed))
        }
    }
}
