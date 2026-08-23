//! `ls` — list directory contents, supporting `-l`, `-a` and `-F`.

use std::collections::HashMap;
use std::fs::{self, Metadata};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::color;
use crate::commands::{CmdResult, Io};

use platform::MetaExt;

/// Parsed command-line options for `ls`.
#[derive(Clone, Copy, Default)]
struct Flags {
    long: bool,     // -l : long listing format
    all: bool,      // -a : include entries starting with '.'
    classify: bool, // -F : append an indicator (one of */=>@|) to entries
}

/// Entry point for the `ls` built-in.
pub fn run(args: &[String], io: &mut Io) -> CmdResult {
    let mut flags = Flags::default();
    let mut operands: Vec<&String> = Vec::new();

    for arg in args {
        if arg.starts_with('-') && arg.len() > 1 {
            for ch in arg.chars().skip(1) {
                match ch {
                    'l' => flags.long = true,
                    'a' => flags.all = true,
                    'F' => flags.classify = true,
                    other => return Err(format!("invalid option -- '{other}'")),
                }
            }
        } else {
            operands.push(arg);
        }
    }

    let default = String::from(".");
    if operands.is_empty() {
        operands.push(&default);
    }

    list_operands(&operands, flags, io)
}

/// A single item to be displayed, together with the metadata needed to render
/// it. Metadata comes from `lstat`, so symlinks are described as links.
struct Entry {
    display: String,
    metadata: Metadata,
    link_target: Option<PathBuf>,
}

impl Entry {
    fn from_path(path: &Path, display: String) -> io::Result<Self> {
        let metadata = fs::symlink_metadata(path)?;
        let link_target = if metadata.file_type().is_symlink() {
            fs::read_link(path).ok()
        } else {
            None
        };
        Ok(Entry {
            display,
            metadata,
            link_target,
        })
    }
}

/// Lists every operand. File operands are grouped and printed first; directory
/// operands are then expanded, each with a header when more than one target is
/// involved.
fn list_operands(operands: &[&String], flags: Flags, io: &mut Io) -> CmdResult {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let resolver = NameResolver::new();

    let mut files: Vec<Entry> = Vec::new();
    let mut dirs: Vec<&String> = Vec::new();

    for operand in operands {
        match fs::symlink_metadata(Path::new(operand)) {
            Ok(meta) if meta.is_dir() => dirs.push(operand),
            Ok(_) => match Entry::from_path(Path::new(operand), (*operand).clone()) {
                Ok(entry) => files.push(entry),
                Err(e) => eprintln!("ls: cannot access '{operand}': {e}"),
            },
            Err(e) => eprintln!("ls: cannot access '{operand}': {e}"),
        }
    }

    let show_headers = operands.len() > 1;
    let mut first_block = true;

    if !files.is_empty() {
        render(&files, flags, &resolver, now, io).map_err(|e| e.to_string())?;
        first_block = false;
    }

    for dir in dirs {
        if !first_block {
            writeln!(io.out).map_err(|e| e.to_string())?;
        }
        first_block = false;

        if show_headers {
            writeln!(io.out, "{dir}:").map_err(|e| e.to_string())?;
        }
        if let Err(e) = list_directory(Path::new(dir), flags, &resolver, now, io) {
            eprintln!("ls: cannot open directory '{dir}': {e}");
        }
    }

    Ok(())
}

/// Reads a directory, builds its entry list (honouring `-a`), sorts it, and
/// renders it.
fn list_directory(
    dir: &Path,
    flags: Flags,
    resolver: &NameResolver,
    now: i64,
    io: &mut Io,
) -> io::Result<()> {
    let mut entries: Vec<Entry> = Vec::new();

    if flags.all {
        // The synthetic "." and ".." entries only appear with -a.
        for (name, path) in [(".", dir.to_path_buf()), ("..", dir.join(".."))] {
            if let Ok(entry) = Entry::from_path(&path, name.to_string()) {
                entries.push(entry);
            }
        }
    }

    for dir_entry in fs::read_dir(dir)? {
        let dir_entry = dir_entry?;
        let name = dir_entry.file_name().to_string_lossy().into_owned();
        if !flags.all && name.starts_with('.') {
            continue;
        }
        if let Ok(entry) = Entry::from_path(&dir_entry.path(), name) {
            entries.push(entry);
        }
    }

    entries.sort_by(|a, b| sort_key(&a.display).cmp(&sort_key(&b.display)));

    if flags.long {
        let total: u64 = entries.iter().map(|e| e.metadata.blocks()).sum();
        // st_blocks counts 512-byte units; ls reports 1024-byte blocks.
        writeln!(io.out, "total {}", total / 2)?;
    }

    render(&entries, flags, resolver, now, io)
}

/// Renders a slice of entries in either long or short form.
fn render(
    entries: &[Entry],
    flags: Flags,
    resolver: &NameResolver,
    now: i64,
    io: &mut Io,
) -> io::Result<()> {
    if flags.long {
        render_long(entries, flags, resolver, now, io)
    } else {
        for entry in entries {
            writeln!(io.out, "{}", decorate(entry, flags, io.color))?;
        }
        Ok(())
    }
}

/// Renders entries in `-l` long format with columns aligned to their widest
/// value, mirroring GNU `ls`.
fn render_long(
    entries: &[Entry],
    flags: Flags,
    resolver: &NameResolver,
    now: i64,
    io: &mut Io,
) -> io::Result<()> {
    struct Row {
        perms: String,
        links: String,
        owner: String,
        group: String,
        size: String,
        time: String,
        name: String,
    }

    let color = io.color;
    let rows: Vec<Row> = entries
        .iter()
        .map(|e| {
            let meta = &e.metadata;
            let name = match &e.link_target {
                Some(target) => {
                    format!("{} -> {}", decorate(e, flags, color), target.display())
                }
                None => decorate(e, flags, color),
            };
            Row {
                perms: permission_string(meta.mode()),
                links: meta.nlink().to_string(),
                owner: resolver.user(meta.uid()),
                group: resolver.group(meta.gid()),
                size: meta.size().to_string(),
                time: format_time(meta.mtime(), now),
                name,
            }
        })
        .collect();

    let w_links = max_width(rows.iter().map(|r| r.links.len()));
    let w_owner = max_width(rows.iter().map(|r| r.owner.len()));
    let w_group = max_width(rows.iter().map(|r| r.group.len()));
    let w_size = max_width(rows.iter().map(|r| r.size.len()));

    for r in &rows {
        writeln!(
            io.out,
            "{} {:>w_links$} {:<w_owner$} {:<w_group$} {:>w_size$} {} {}",
            r.perms,
            r.links,
            r.owner,
            r.group,
            r.size,
            r.time,
            r.name,
            w_links = w_links,
            w_owner = w_owner,
            w_group = w_group,
            w_size = w_size,
        )?;
    }
    Ok(())
}

/// Returns the display name with a `-F` type indicator and/or colour applied.
fn decorate(entry: &Entry, flags: Flags, color: bool) -> String {
    let mode = entry.metadata.mode();
    let file_type = mode & 0o170000;

    let mut name = entry.display.clone();

    if color {
        let code = match file_type {
            0o040000 => Some(color::BOLD_BLUE),              // directory
            0o120000 => Some(color::BOLD_CYAN),              // symlink
            0o100000 if mode & 0o111 != 0 => Some(color::BOLD_GREEN), // executable
            _ => None,
        };
        if let Some(code) = code {
            name = color::paint(true, code, &name);
        }
    }

    if flags.classify {
        let suffix = match file_type {
            0o040000 => "/",                      // directory
            0o120000 => "@",                      // symbolic link
            0o010000 => "|",                      // FIFO
            0o140000 => "=",                      // socket
            0o100000 if mode & 0o111 != 0 => "*", // executable file
            _ => "",
        };
        name.push_str(suffix);
    }

    name
}

/// Produces a sort key that mimics `ls`: comparison ignores a leading dot and
/// is case-insensitive, which matches GNU's default collation closely enough
/// for everyday use.
fn sort_key(name: &str) -> String {
    name.trim_start_matches('.').to_lowercase()
}

fn max_width(widths: impl Iterator<Item = usize>) -> usize {
    widths.max().unwrap_or(0)
}

/// Builds the 10-character permission string, e.g. `drwxr-xr-x`.
fn permission_string(mode: u32) -> String {
    let file_type = match mode & 0o170000 {
        0o040000 => 'd',
        0o120000 => 'l',
        0o020000 => 'c',
        0o060000 => 'b',
        0o010000 => 'p',
        0o140000 => 's',
        _ => '-',
    };

    let mut s = String::with_capacity(10);
    s.push(file_type);
    s.push(rwx_bit(mode & 0o400 != 0, 'r'));
    s.push(rwx_bit(mode & 0o200 != 0, 'w'));
    s.push(exec_bit(mode & 0o100 != 0, mode & 0o4000 != 0, 's'));
    s.push(rwx_bit(mode & 0o040 != 0, 'r'));
    s.push(rwx_bit(mode & 0o020 != 0, 'w'));
    s.push(exec_bit(mode & 0o010 != 0, mode & 0o2000 != 0, 's'));
    s.push(rwx_bit(mode & 0o004 != 0, 'r'));
    s.push(rwx_bit(mode & 0o002 != 0, 'w'));
    s.push(exec_bit(mode & 0o001 != 0, mode & 0o1000 != 0, 't'));
    s
}

fn rwx_bit(set: bool, ch: char) -> char {
    if set {
        ch
    } else {
        '-'
    }
}

/// Renders an execute position, folding in the setuid/setgid/sticky bit.
fn exec_bit(executable: bool, special: bool, special_char: char) -> char {
    match (executable, special) {
        (true, true) => special_char,
        (false, true) => special_char.to_ascii_uppercase(),
        (true, false) => 'x',
        (false, false) => '-',
    }
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Formats a modification time the way `ls -l` does: `Mon DD HH:MM` for recent
/// files and `Mon DD  YYYY` for files older than roughly six months.
///
/// Times are computed in UTC, which is consistent inside the container.
fn format_time(mtime: i64, now: i64) -> String {
    let (year, month, day) = civil_from_days(mtime.div_euclid(86400));
    let seconds_of_day = mtime.rem_euclid(86400);
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;

    let month_name = MONTHS[(month - 1) as usize];

    // Roughly six months, matching coreutils' recency heuristic.
    const SIX_MONTHS: i64 = 15_552_000;
    if (now - mtime).abs() > SIX_MONTHS {
        format!("{month_name} {day:>2}  {year}")
    } else {
        format!("{month_name} {day:>2} {hour:02}:{minute:02}")
    }
}

/// Converts a count of days since the Unix epoch into a civil `(year, month,
/// day)` triple using Howard Hinnant's well-known algorithm.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (year + i64::from(month <= 2), month, day)
}

/// Resolves numeric user and group IDs to names by reading `/etc/passwd` and
/// `/etc/group`. Falls back to the numeric ID when a name is unavailable.
struct NameResolver {
    users: HashMap<u32, String>,
    groups: HashMap<u32, String>,
}

impl NameResolver {
    fn new() -> Self {
        let mut users = parse_id_file("/etc/passwd");
        let mut groups = parse_id_file("/etc/group");
        // On platforms without a passwd database every entry reports id 0;
        // give that id a meaningful name instead of printing a bare "0".
        if let Some(name) = platform::fallback_owner() {
            users.entry(0).or_insert_with(|| name.clone());
            groups.entry(0).or_insert(name);
        }
        NameResolver { users, groups }
    }

    fn user(&self, uid: u32) -> String {
        self.users
            .get(&uid)
            .cloned()
            .unwrap_or_else(|| uid.to_string())
    }

    fn group(&self, gid: u32) -> String {
        self.groups
            .get(&gid)
            .cloned()
            .unwrap_or_else(|| gid.to_string())
    }
}

/// Parses a colon-separated `passwd`/`group`-style file into an `id -> name`
/// map. Both formats put the name in field 0 and the numeric id in field 2.
fn parse_id_file(path: &str) -> HashMap<u32, String> {
    let mut map = HashMap::new();
    if let Ok(contents) = fs::read_to_string(path) {
        for line in contents.lines() {
            let fields: Vec<&str> = line.split(':').collect();
            if fields.len() >= 3 {
                if let Ok(id) = fields[2].parse::<u32>() {
                    map.entry(id).or_insert_with(|| fields[0].to_string());
                }
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_string_regular_file() {
        // 0o100644 -> regular file, rw-r--r--
        assert_eq!(permission_string(0o100_644), "-rw-r--r--");
    }

    #[test]
    fn permission_string_directory() {
        assert_eq!(permission_string(0o040_755), "drwxr-xr-x");
    }

    #[test]
    fn permission_string_setuid() {
        assert_eq!(permission_string(0o104_755), "-rwsr-xr-x");
    }

    #[test]
    fn civil_from_days_epoch() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
    }

    #[test]
    fn civil_from_days_known_date() {
        // 2000-01-01 is 10957 days after the epoch.
        assert_eq!(civil_from_days(10_957), (2000, 1, 1));
    }
}

/// Platform shims for the file metadata `ls -l` renders.
///
/// On Unix these map straight onto the `stat` fields. Elsewhere they are
/// synthesised from the portable parts of `Metadata`, so the rendering code
/// above stays platform-independent and the shell still builds outside the
/// Docker image.
mod platform {
    use std::fs::Metadata;

    /// The subset of `stat` fields the long listing needs.
    pub trait MetaExt {
        fn mode(&self) -> u32;
        fn nlink(&self) -> u64;
        fn uid(&self) -> u32;
        fn gid(&self) -> u32;
        fn size(&self) -> u64;
        fn mtime(&self) -> i64;
        fn blocks(&self) -> u64;
    }

    #[cfg(unix)]
    impl MetaExt for Metadata {
        fn mode(&self) -> u32 {
            std::os::unix::fs::MetadataExt::mode(self)
        }
        fn nlink(&self) -> u64 {
            std::os::unix::fs::MetadataExt::nlink(self)
        }
        fn uid(&self) -> u32 {
            std::os::unix::fs::MetadataExt::uid(self)
        }
        fn gid(&self) -> u32 {
            std::os::unix::fs::MetadataExt::gid(self)
        }
        fn size(&self) -> u64 {
            std::os::unix::fs::MetadataExt::size(self)
        }
        fn mtime(&self) -> i64 {
            std::os::unix::fs::MetadataExt::mtime(self)
        }
        fn blocks(&self) -> u64 {
            std::os::unix::fs::MetadataExt::blocks(self)
        }
    }

    /// A `passwd` database exists, so no substitute name is needed.
    #[cfg(unix)]
    pub fn fallback_owner() -> Option<String> {
        None
    }

    #[cfg(not(unix))]
    impl MetaExt for Metadata {
        /// Synthesises a POSIX mode word: the file type comes from
        /// `file_type()` and the permission bits from the read-only flag,
        /// which is how WSL and Git for Windows present the same files.
        fn mode(&self) -> u32 {
            let file_type = self.file_type();
            let readonly = self.permissions().readonly();
            if file_type.is_dir() {
                0o040000 | if readonly { 0o555 } else { 0o755 }
            } else if file_type.is_symlink() {
                0o120000 | 0o777
            } else {
                0o100000 | if readonly { 0o444 } else { 0o644 }
            }
        }

        /// Hard-link counts are not exposed by `Metadata` off Unix.
        fn nlink(&self) -> u64 {
            1
        }

        fn uid(&self) -> u32 {
            0
        }

        fn gid(&self) -> u32 {
            0
        }

        fn size(&self) -> u64 {
            self.len()
        }

        fn mtime(&self) -> i64 {
            use std::time::UNIX_EPOCH;
            match self.modified() {
                Ok(t) => match t.duration_since(UNIX_EPOCH) {
                    Ok(d) => d.as_secs() as i64,
                    Err(e) => -(e.duration().as_secs() as i64),
                },
                Err(_) => 0,
            }
        }

        /// `st_blocks` counts 512-byte units; round the logical size up.
        fn blocks(&self) -> u64 {
            (self.len() + 511) / 512
        }
    }

    /// Without a `passwd` database every entry reports id 0, so fall back to
    /// the logged-in account name for the owner and group columns.
    #[cfg(not(unix))]
    pub fn fallback_owner() -> Option<String> {
        std::env::var("USERNAME").ok().filter(|n| !n.is_empty())
    }
}
