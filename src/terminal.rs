//! Thin, hand-written bindings to the few terminal facilities the shell
//! needs: a tty test, raw mode, and ignoring the interrupt key.
//!
//! These live in libraries Rust already links against, so no external crate is
//! required and — importantly for this project — no external binary is ever
//! spawned.
//!
//! Two backends are provided. The POSIX one uses `isatty`/`termios`/`signal`
//! and matches the Linux/glibc layout used by the accompanying Docker image,
//! which is the primary target. The Windows one drives the console through
//! `GetConsoleMode`/`SetConsoleMode` so the shell can also be built and run
//! natively on a development machine.

#![allow(non_camel_case_types)]

use std::os::raw::c_int;

pub const STDIN_FILENO: c_int = 0;
pub const STDOUT_FILENO: c_int = 1;
pub const STDERR_FILENO: c_int = 2;

#[cfg(unix)]
mod imp {
    use std::mem;
    use std::os::raw::c_int;

    use super::STDIN_FILENO;

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

                // Disable canonical mode and echo so we receive keystrokes as
                // they are typed, and disable signal generation so Ctrl+C and
                // Ctrl+Z reach us as ordinary bytes. IXON is cleared so Ctrl+S
                // does not freeze input.
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
}

#[cfg(windows)]
mod imp {
    use std::os::raw::c_int;

    use super::{STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO};

    type BOOL = i32;
    type DWORD = u32;
    /// `HANDLE` is a pointer-sized opaque value. Keeping it as an integer means
    /// the guard below stays a plain struct with no raw pointers in it.
    type HANDLE = isize;

    const TRUE: BOOL = 1;
    const INVALID_HANDLE_VALUE: HANDLE = -1;
    const STD_INPUT_HANDLE: DWORD = -10i32 as DWORD;
    const STD_OUTPUT_HANDLE: DWORD = -11i32 as DWORD;
    const STD_ERROR_HANDLE: DWORD = -12i32 as DWORD;

    // Console input mode bits.
    const ENABLE_PROCESSED_INPUT: DWORD = 0x0001;
    const ENABLE_LINE_INPUT: DWORD = 0x0002;
    const ENABLE_ECHO_INPUT: DWORD = 0x0004;
    const ENABLE_VIRTUAL_TERMINAL_INPUT: DWORD = 0x0200;
    // Console output mode bits.
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: DWORD = 0x0004;

    extern "system" {
        fn GetStdHandle(nStdHandle: DWORD) -> HANDLE;
        fn GetConsoleMode(hConsoleHandle: HANDLE, lpMode: *mut DWORD) -> BOOL;
        fn SetConsoleMode(hConsoleHandle: HANDLE, dwMode: DWORD) -> BOOL;
        fn SetConsoleCtrlHandler(HandlerRoutine: usize, Add: BOOL) -> BOOL;
    }

    /// Maps one of the POSIX descriptor numbers the rest of the shell uses onto
    /// the corresponding standard console handle.
    fn std_handle(fd: c_int) -> HANDLE {
        let which = match fd {
            STDIN_FILENO => STD_INPUT_HANDLE,
            STDOUT_FILENO => STD_OUTPUT_HANDLE,
            STDERR_FILENO => STD_ERROR_HANDLE,
            _ => return INVALID_HANDLE_VALUE,
        };
        unsafe { GetStdHandle(which) }
    }

    /// Reads a console mode, or `None` when the handle is not a console.
    fn console_mode(handle: HANDLE) -> Option<DWORD> {
        if handle == INVALID_HANDLE_VALUE || handle == 0 {
            return None;
        }
        let mut mode: DWORD = 0;
        if unsafe { GetConsoleMode(handle, &mut mode) } == 0 {
            None
        } else {
            Some(mode)
        }
    }

    /// Returns whether the given descriptor is connected to a terminal.
    ///
    /// `GetConsoleMode` only succeeds on a real console handle, so it fails for
    /// redirected files and pipes exactly where `isatty` would return 0.
    pub fn is_tty(fd: c_int) -> bool {
        console_mode(std_handle(fd)).is_some()
    }

    /// Instructs the process to ignore Ctrl+C.
    ///
    /// Registering a null handler is the documented way to make a console
    /// process ignore the interrupt, and is the counterpart of `SIG_IGN`. As on
    /// Unix, raw mode then delivers Ctrl+C to the line editor as a plain byte.
    pub fn ignore_sigint() {
        unsafe {
            SetConsoleCtrlHandler(0, TRUE);
        }
    }

    /// An RAII guard that puts the console into raw mode and restores the
    /// previous settings when dropped.
    pub struct RawMode {
        input: HANDLE,
        input_mode: DWORD,
        /// Set only when this guard was the one to switch VT output on, so drop
        /// restores exactly what it changed.
        output: Option<(HANDLE, DWORD)>,
    }

    impl RawMode {
        /// Enables raw mode on the console. Returns `None` if the mode could
        /// not be read or applied (for example when stdin is not a console).
        pub fn enable() -> Option<RawMode> {
            let input = std_handle(STDIN_FILENO);
            let input_mode = console_mode(input)?;

            // Deliver each keystroke immediately rather than a line at a time,
            // do not echo it, and let Ctrl+C arrive as a byte instead of
            // raising a control event. Virtual-terminal input makes the console
            // report arrow and navigation keys as the same escape sequences the
            // line editor already parses on Unix.
            let raw = (input_mode
                & !(ENABLE_PROCESSED_INPUT | ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT))
                | ENABLE_VIRTUAL_TERMINAL_INPUT;
            if unsafe { SetConsoleMode(input, raw) } == 0 {
                return None;
            }

            // The editor redraws with ANSI escapes, which the classic console
            // host only honours once VT processing is switched on. Leaving it
            // off is not fatal — Windows Terminal enables it already — so a
            // failure here is ignored.
            let output = std_handle(STDOUT_FILENO);
            let restore_output = match console_mode(output) {
                Some(mode) if mode & ENABLE_VIRTUAL_TERMINAL_PROCESSING == 0 => {
                    let vt = mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
                    if unsafe { SetConsoleMode(output, vt) } == 0 {
                        None
                    } else {
                        Some((output, mode))
                    }
                }
                _ => None,
            };

            Some(RawMode {
                input,
                input_mode,
                output: restore_output,
            })
        }
    }

    impl Drop for RawMode {
        fn drop(&mut self) {
            unsafe {
                SetConsoleMode(self.input, self.input_mode);
                if let Some((handle, mode)) = self.output {
                    SetConsoleMode(handle, mode);
                }
            }
        }
    }
}

pub use imp::{ignore_sigint, is_tty, RawMode};
