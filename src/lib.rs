#![warn(rust_2018_idioms)]

extern crate libsystemd_sys as ffi;

/*
extern crate enumflags2;
#[macro_use]
extern crate enumflags2_derive;
*/

#[cfg(feature = "journal")]
pub use journal::{
    Journal, JournalFiles, JournalLog, JournalRecord, JournalSeek, JournalWaitResult,
};
use libc::{c_char, c_void, free, strlen};
pub use std::io::{Error, Result};

#[cfg(any(feature = "journal", feature = "bus"))]
fn usec_from_duration(duration: std::time::Duration) -> u64 {
    let sub_usecs = duration.subsec_micros() as u64;
    duration.as_secs() * 1_000_000 + sub_usecs
}

/// Convert a systemd ffi return value into a Result
pub fn ffi_result(ret: ffi::c_int) -> Result<ffi::c_int> {
    if ret < 0 {
        Err(Error::from_raw_os_error(-ret))
    } else {
        Ok(ret)
    }
}

/// Convert a malloc'd C string into a rust string and call free on it.
/// Returns None if the pointer is null.
unsafe fn free_cstring(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let len = strlen(ptr);
    let char_slice = std::slice::from_raw_parts(ptr as *mut u8, len);
    let s = String::from_utf8_lossy(&char_slice).into_owned();
    free(ptr as *mut c_void);
    Some(s)
}

/// An analogue of `try!()` for systemd FFI calls.
///
/// The parameter should be a call to a systemd FFI fn with an c_int return
/// value. It is called, and if the return is negative then `sd_try!()`
/// interprets it as an error code and returns IoError from the enclosing fn.
/// Otherwise, the value of `sd_try!()` is the non-negative value returned by
/// the FFI call.
#[macro_export]
macro_rules! sd_try {
    ($e:expr) => {{
        $crate::ffi_result(unsafe { $e })?
    }};
}

/// High-level interface to the systemd journal.
///
/// The main interface for writing to the journal is `fn log()`, and the main
/// interface for reading the journal is `struct Journal`.
#[cfg(feature = "journal")]
pub mod journal;

/// Similar to `log!()`, except it accepts a func argument rather than hard
/// coding `::log::log()`, and it doesn't filter on `log_enabled!()`.
#[macro_export]
macro_rules! log_with{
    ($func:expr, $lvl:expr, $($arg:tt),+) => ({
        $func(&::log::Record::builder()
            .args(format_args!($($arg),+))
            .level($lvl)
            .file(Some(file!()))
            .line(Some(line!()))
            .module_path(Some(module_path!()))
            .build())
    });
    (@raw $func:expr, $lvl:expr, $($arg:tt),+) => ({
        $func($lvl, file!(), line!(), module_path!(), &format_args!($($arg),+))
    });
    (@target $tgt:expr, $func:expr, $lvl:expr, $($arg:tt),+) => ({
        $func(&::log::Record::builder()
            .args(format_args!($($arg),+))
            .level($lvl)
            .target($tgt)
            .file(Some(file!()))
            .line(Some(line!()))
            .module_path(Some(module_path!()))
            .build())
    })
}

#[cfg(feature = "journal")]
#[macro_export]
macro_rules! sd_journal_log{
    ($lvl:expr, $($arg:tt)+) => ($crate::log_with!(@raw ::systemd::journal::log, $lvl, $($arg)+))
}

pub mod daemon;

pub mod id128;

/// Interface to introspect on seats, sessions and users.
pub mod login;

/// An interface to work with the dbus message bus.
///
#[cfg(feature = "bus")]
pub mod bus;
