//! In-process output sink.
//!
//! Normally the calculators write straight to stdout (results) and stderr
//! (warnings / diagnostics), exactly as before. Inside [`capture`], that output
//! is redirected into two in-memory strings instead — this lets the MCP server
//! (`puc mcp-server`) collect a subcommand's output as a tool result without
//! corrupting the stdio JSON-RPC stream it owns.
//!
//! Call sites use the [`out!`](crate::out), [`outln!`](crate::outln) and
//! [`errln!`](crate::errln) macros in place of `print!` / `println!` /
//! `eprintln!`; no function signatures carry a writer.

use std::cell::RefCell;
use std::fmt::Arguments;

#[derive(Default)]
struct Capture {
    res: String,
    diag: String,
}

thread_local! {
    static CAPTURE: RefCell<Option<Capture>> = const { RefCell::new(None) };
}

/// Run `f` with result/diagnostic output captured into `(result, diagnostics)`
/// strings instead of stdout/stderr, returning `(f's return, result, diag)`.
///
/// Must run start-to-finish on one thread (no `.await` between set and take);
/// the calculators are synchronous so the MCP handlers satisfy this. Not
/// re-entrant — the codebase never nests captures.
pub fn capture<R>(f: impl FnOnce() -> R) -> (R, String, String) {
    CAPTURE.with(|c| *c.borrow_mut() = Some(Capture::default()));
    let r = f();
    let cap = CAPTURE.with(|c| c.borrow_mut().take()).unwrap_or_default();
    (r, cap.res, cap.diag)
}

#[doc(hidden)]
pub fn emit_res(args: Arguments) {
    CAPTURE.with(|c| {
        if let Some(cap) = c.borrow_mut().as_mut() {
            use std::fmt::Write;
            let _ = cap.res.write_fmt(args);
        } else {
            use std::io::Write;
            let _ = std::io::stdout().write_fmt(args);
        }
    });
}

#[doc(hidden)]
pub fn emit_diag(args: Arguments) {
    CAPTURE.with(|c| {
        if let Some(cap) = c.borrow_mut().as_mut() {
            use std::fmt::Write;
            let _ = cap.diag.write_fmt(args);
        } else {
            use std::io::Write;
            let _ = std::io::stderr().write_fmt(args);
        }
    });
}

/// `print!` to the result stream (stdout, or the capture buffer).
#[macro_export]
macro_rules! out {
    ($($arg:tt)*) => { $crate::output::emit_res(::std::format_args!($($arg)*)) };
}

/// `println!` to the result stream (stdout, or the capture buffer).
#[macro_export]
macro_rules! outln {
    () => { $crate::output::emit_res(::std::format_args!("\n")) };
    ($($arg:tt)*) => {
        $crate::output::emit_res(::std::format_args!("{}\n", ::std::format_args!($($arg)*)))
    };
}

/// `eprintln!` to the diagnostic stream (stderr, or the capture buffer).
#[macro_export]
macro_rules! errln {
    () => { $crate::output::emit_diag(::std::format_args!("\n")) };
    ($($arg:tt)*) => {
        $crate::output::emit_diag(::std::format_args!("{}\n", ::std::format_args!($($arg)*)))
    };
}
