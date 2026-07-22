//! In-memory command history with optional persistence to a file in `$HOME`.

use std::env;
use std::fs;
use std::path::PathBuf;

const HISTORY_FILE: &str = ".0-shell_history";
const MAX_ENTRIES: usize = 1000;

/// The command history. Entries are ordered oldest-first.
pub struct History {
    entries: Vec<String>,
    path: Option<PathBuf>,
}

impl History {
    /// Loads history from `$HOME/.0-shell_history`, if present.
    pub fn load() -> History {
        let path = env::var_os("HOME").map(|home| PathBuf::from(home).join(HISTORY_FILE));

        let entries = path
            .as_ref()
            .and_then(|p| fs::read_to_string(p).ok())
            .map(|contents| contents.lines().map(str::to_string).collect())
            .unwrap_or_default();

        History { entries, path }
    }

    /// Returns the recorded entries, oldest first.
    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    /// Records a command line, ignoring blanks and immediate duplicates.
    pub fn add(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        if self.entries.last().map(String::as_str) == Some(line) {
            return;
        }
        self.entries.push(line.to_string());
        if self.entries.len() > MAX_ENTRIES {
            let overflow = self.entries.len() - MAX_ENTRIES;
            self.entries.drain(0..overflow);
        }
    }

    /// Writes the history back to disk. Failures are ignored: a missing or
    /// unwritable history file must never take the shell down.
    pub fn save(&self) {
        if let Some(path) = &self.path {
            let _ = fs::write(path, self.entries.join("\n"));
        }
    }
}
