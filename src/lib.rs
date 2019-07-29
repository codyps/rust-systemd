extern crate cstr_argument;
extern crate libc;
extern crate libsystemd_sys as ffi;
extern crate log;

/*
extern crate enumflags2;
#[macro_use]
extern crate enumflags2_derive;
*/

use libc::{c_char, c_void, free, strlen};
pub use std::io::{Error, Result};

fn usec_from_duration(duration: std::time::Duration) -> u64 {
    let sub_usecs = (duration.subsec_nanos() / 1000) as u64;
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
fn free_cstring(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe {
        let len = strlen(ptr);
        let char_slice = std::slice::from_raw_parts(ptr as *mut u8, len);
        let s = String::from_utf8_lossy(&char_slice).into_owned();
        free(ptr as *mut c_void);
        Some(s)
    }
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
        try!($crate::ffi_result(unsafe { $e }))
    }};
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

#[macro_export]
macro_rules! sd_journal_log{
    ($lvl:expr, $($arg:tt)+) => (log_with!(@raw ::systemd::journal::log, $lvl, $($arg)+))
}

/// High-level interface to the systemd daemon module.
pub mod daemon;

pub mod id128;

/// Interface to introspect on seats, sessions and users.
pub mod login;

/// An interface to work with the dbus message bus.
///
#[cfg(feature = "bus")]
pub mod bus;
