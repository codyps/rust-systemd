#![feature(libc,std_misc,collections,core)]
#![unstable]

extern crate collections;
extern crate libc;
#[macro_use] extern crate log;
extern crate "libsystemd-sys" as ffi;

use std::result;

pub enum Error {
    Errno(libc::c_int)
}
pub type Result<T> = result::Result<T, Error>;

/// An analogue of `try!()` for systemd FFI calls.
///
/// The parameter should be a call to a systemd FFI fn with an i32 return
/// value. It is called, and if the return is negative then `sd_try!()`
/// interprets it as an error code and returns IoError from the enclosing fn.
/// Otherwise, the value of `sd_try!()` is the non-negative value returned by
/// the FFI call.
#[macro_export]
macro_rules! sd_try {
    ($e:expr) => ({
        let ret: i32;
        unsafe {
            ret = $e;
        }
        if ret < 0 {
            return Err(Error::Errno(-ret));
        }
        ret
    })
}

/// Given an Option<&str>, either returns a pointer to a const char*, or a NULL
/// pointer if None.
#[macro_export]
macro_rules! char_or_null {
    ($e:expr) => (match $e {
        Some(p) => ::std::ffi::CString::from_slice(p.as_bytes()).as_ptr(),
        None => ptr::null()
    })
}

/// High-level interface to the systemd journal.
///
/// The main interface for writing to the journal is `fn log()`, and the main
/// interface for reading the journal is `struct Journal`.
#[unstable]
pub mod journal;

/// Similar to `log!()`, except it accepts a func argument rather than hard
/// coding `::log::log()`, and it doesn't filter on `log_enabled!()`.
#[macro_export]
macro_rules! log_with{
    ($func:expr, $lvl:expr, $($arg:tt),+) => ({
        static LOC: ::log::LogLocation = ::log::LogLocation {
            line: line!(),
            file: file!(),
            module_path: module_path!()
        };
        let lvl = $lvl;
        let func = $func;
        $func(lvl, &LOC, &format_args!($($arg),+))
    })
}

#[macro_export]
macro_rules! sd_journal_log{
    ($lvl:expr, $($arg:tt)+) => (log_with!(::systemd::journal::log, $lvl, $($arg)+))
}

/// High-level interface to the systemd daemon module.
#[unstable]
pub mod daemon;
