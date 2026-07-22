//! Thin, hand-written bindings to the few POSIX terminal facilities the shell
//! needs: `isatty`, raw-mode `termios`, and ignoring `SIGINT`.
//!
//! These live in the C library that Rust already links against, so no external
//! crate is required and — importantly for this project — no external binary
//! is ever spawned.
//!
//! The struct layout and flag values match Linux/glibc, which is the target
//! for the accompanying Docker image.

#![allow(non_camel_case_types)]

use std::mem;
use std::os::raw::c_int;

pub const STDIN_FILENO: c_int = 0;
pub const STDOUT_FILENO: c_int = 1;
pub const STDERR_FILENO: c_int = 2;

const NCCS: usize = 32;

// c_lflag bits.
const ICANON: u32 = 0o0000002;
const ECHO: u32 = 0o0000010;
const ISIG: u32 = 0o0000001;
// c_iflag bits.
const IXON: u32 = 0o0002000;
// c_cc indices.
const VTIME: usize = 5;
const VMIN: usize = 6;

const TCSANOW: c_int = 0;
const SIGINT: c_int = 2;
const SIG_IGN: usize = 1;

/// Mirrors `struct termios` on Linux.
#[repr(C)]
#[derive(Clone, Copy)]
struct Termios {
    c_iflag: u32,
    c_oflag: u32,
    c_cflag: u32,
    c_lflag: u32,
    c_line: u8,
    c_cc: [u8; NCCS],
    c_ispeed: u32,
    c_ospeed: u32,
}

extern "C" {
    fn isatty(fd: c_int) -> c_int;
    fn tcgetattr(fd: c_int, termios_p: *mut Termios) -> c_int;
    fn tcsetattr(fd: c_int, optional_actions: c_int, termios_p: *const Termios) -> c_int;
    fn signal(signum: c_int, handler: usize) -> usize;
}

/// Returns whether the given file descriptor is connected to a terminal.
pub fn is_tty(fd: c_int) -> bool {
    unsafe { isatty(fd) == 1 }
}

/// Instructs the process to ignore `SIGINT` (Ctrl+C).
///
/// This guarantees the shell itself never dies from Ctrl+C. During
/// line-editing the terminal is in raw mode, so Ctrl+C arrives as a byte we
/// handle ourselves; while a built-in runs, the signal is simply ignored.
pub fn ignore_sigint() {
    unsafe {
        signal(SIGINT, SIG_IGN);
    }
}

/// An RAII guard that puts stdin into raw mode and restores the previous
/// settings when dropped.
pub struct RawMode {
    original: Termios,
}

impl RawMode {
    /// Enables raw mode on stdin. Returns `None` if the terminal settings
    /// could not be read or applied (for example when stdin is not a tty).
    pub fn enable() -> Option<RawMode> {
        unsafe {
            let mut term: Termios = mem::zeroed();
            if tcgetattr(STDIN_FILENO, &mut term) != 0 {
                return None;
            }
            let original = term;

            // Disable canonical mode and echo so we receive keystrokes as they
            // are typed, and disable signal generation so Ctrl+C/Ctrl+Z reach
            // us as ordinary bytes. IXON is cleared so Ctrl+S does not freeze
            // input.
            term.c_lflag &= !(ICANON | ECHO | ISIG);
            term.c_iflag &= !IXON;
            term.c_cc[VMIN] = 1; // block until at least one byte is available
            term.c_cc[VTIME] = 0; // with no inter-byte timeout

            if tcsetattr(STDIN_FILENO, TCSANOW, &term) != 0 {
                return None;
            }
            Some(RawMode { original })
        }
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        unsafe {
            tcsetattr(STDIN_FILENO, TCSANOW, &self.original);
        }
    }
}
