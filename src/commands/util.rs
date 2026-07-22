//! Small helpers shared between built-ins.

use std::ffi::OsStr;
use std::path::Path;

/// Returns the final component of a path (its file name), if any.
///
/// Unlike [`Path::file_name`], a trailing-slash path such as `foo/` still
/// yields `foo`, which is what the copy/move destination logic wants.
pub fn file_name(path: &Path) -> Option<&OsStr> {
    path.file_name()
}
