extern crate libc;
#[macro_use] extern crate log;
extern crate libsystemd_sys as ffi;
pub use std::io::{Result, Error};

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
            return Err($crate::Error::from_raw_os_error(-ret));
        }
        ret
    })
}

/// Given an Option<&str>, either returns a pointer to a const char*, or a NULL
/// pointer if None.
#[macro_export]
macro_rules! char_or_null {
    ($e:expr) => (match $e {
        Some(p) => ::std::ffi::CString::new(p.as_bytes()).unwrap().as_ptr() as *const ::libc::c_char,
        None => ptr::null() as *const ::libc::c_char
    })
}

/// High-level interface to the systemd journal.
///
/// The main interface for writing to the journal is `fn log()`, and the main
/// interface for reading the journal is `struct Journal`.
pub mod journal;

/// Similar to `log!()`, except it accepts a func argument rather than hard
/// coding `::log::log()`, and it doesn't filter on `log_enabled!()`.
#[macro_export]
macro_rules! log_with{
    ($func:expr, $lvl:expr, $($arg:tt),+) => ({
        static LOC: ::log::LogLocation = ::log::LogLocation {
            __line: line!(),
            __file: file!(),
            __module_path: module_path!()
        };
        let lvl = $lvl;
        $func(lvl, &LOC, &format_args!($($arg),+))
    })
}

#[macro_export]
macro_rules! sd_journal_log{
    ($lvl:expr, $($arg:tt)+) => (log_with!(::systemd::journal::log, $lvl, $($arg)+))
}

/// High-level interface to the systemd daemon module.
pub mod daemon;

/// API for working with 128-bit ID values, which are a generalizastion of OSF UUIDs (see `man 3
/// sd-id128` for details
pub mod id128;
