#![cfg_attr(feature = "unstable-doc-cfg", feature(doc_cfg))]
#![warn(rust_2018_idioms)]

//! Rust interfaces to libsystemd and compatible libraries.
//!
//! Native APIs introduced in newer releases require explicit Cargo features:
//! `systemd_v240` enables boot-specific application IDs and bus timeout APIs;
//! `systemd_v241` adds the raw close-and-unref helper; `systemd_v245` adds
//! journal namespaces; `systemd_v254` adds session start-time queries. Each
//! version feature includes the earlier version features. The default `bus`
//! feature requires the callback-lifetime APIs from systemd 239. With bus
//! disabled, the base APIs require systemd 233 or equivalent capabilities.
//!
//! The build script checks required native symbols by linking a probe for the
//! target without running it. This includes explicit `SYSTEMD_LIB_DIR` and
//! `SYSTEMD_LIBS` selections and libraries selected by `SYSTEMD_PKG_NAME`,
//! including libelogind. Disable unsupported features or install a library
//! providing the selected APIs when the capability check fails.
//!
//! Fallible systemd APIs use [`Error`] and [`Result`]. The crate-owned error preserves errno
//! and I/O error kinds while adding operation and argument context. See [`Error`] for migration
//! from the previous `std::io::Error` re-export. D-Bus method errors use `bus::Error`
//! when the `bus` feature is enabled; logger installation uses `log::SetLoggerError`.

#[cfg(all(feature = "journal", doctest))]
doc_comment::doctest!("../README.md", readme);

extern crate libsystemd_sys as ffi;

/*
extern crate enumflags2;
#[macro_use]
extern crate enumflags2_derive;
*/

#[cfg(feature = "journal")]
#[allow(deprecated)]
pub use journal::JournalFiles;
#[cfg(feature = "journal")]
pub use journal::{
    FormattedJournalLog, Journal, JournalLog, JournalRecord, JournalSeek, JournalWaitResult,
};
use libc::{c_char, c_void, free, strlen};
mod error;
pub use error::Error;

/// The result of a systemd operation. See [`Error`] for details and I/O conversions.
pub type Result<T> = std::result::Result<T, Error>;

mod cstring;
pub use self::cstring::CStrArgument;

#[cfg(any(feature = "journal", feature = "bus"))]
fn usec_from_duration(duration: std::time::Duration) -> Result<u64> {
    duration
        .as_secs()
        .checked_mul(1_000_000)
        .and_then(|usecs| usecs.checked_add(u64::from(duration.subsec_micros())))
        // UINT64_MAX is libsystemd's infinity sentinel, reserved for None.
        .filter(|&usecs| usecs != u64::MAX)
        .ok_or_else(|| {
            Error::from_raw_os_error(libc::EOVERFLOW)
                .with_operation("convert timeout to microseconds")
                .with_argument("timeout")
        })
}

#[cfg(all(test, any(feature = "journal", feature = "bus")))]
#[test]
fn finite_timeout_conversion_checks_overflow_and_reserves_infinity() {
    use std::time::Duration;
    for (duration, expected) in [
        (Duration::ZERO, 0),
        (Duration::from_nanos(999), 0),
        (Duration::new(2, 123_456_789), 2_123_456),
        (Duration::from_micros(u64::MAX - 1), u64::MAX - 1),
    ] {
        assert_eq!(usec_from_duration(duration).unwrap(), expected);
    }
    for duration in [
        Duration::from_micros(u64::MAX),
        Duration::from_micros(u64::MAX) + Duration::from_micros(1),
        Duration::MAX,
    ] {
        assert_eq!(
            usec_from_duration(duration).unwrap_err().raw_os_error(),
            Some(libc::EOVERFLOW)
        );
    }
}

/// Convert a systemd ffi return value into a Result
pub fn ffi_result(ret: ffi::c_int) -> Result<ffi::c_int> {
    if ret < 0 {
        Err(Error::from_raw_os_error(-ret))
    } else {
        Ok(ret)
    }
}

fn ffi_result_with_context(ret: ffi::c_int, operation: &'static str) -> Result<ffi::c_int> {
    ffi_result(ret).map_err(|error| error.with_operation(operation))
}

/// Convert a malloc'd C string into a rust string and call free on it.
/// Returns None if the pointer is null.
unsafe fn free_cstring(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let len = strlen(ptr);
    let char_slice = std::slice::from_raw_parts(ptr as *mut u8, len);
    let s = String::from_utf8_lossy(char_slice).into_owned();
    free(ptr as *mut c_void);
    Some(s)
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

/// Log a message with source location metadata, returning [`Result<()>`].
///
/// Errors from [`journal::log`] are returned to the caller.
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

/// Utilities for working with systemd units.
pub mod unit;
