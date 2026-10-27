#![cfg(feature = "bus")]

use libc::{c_char, c_int};
use libsystemd_sys::bus::sd_bus_error;
use systemd::bus::Error;
use utf8_cstr::Utf8CStr;

// Isolate the native allocation-failure path without exhausting the test host's
// memory. libsystemd returns this static error when strdup(name) fails.
#[no_mangle]
unsafe extern "C" fn sd_bus_error_set(
    error: *mut sd_bus_error,
    _name: *const c_char,
    _message: *const c_char,
) -> c_int {
    // SAFETY: Error::new passes initialized writable sd_bus_error storage.
    unsafe {
        *error = sd_bus_error {
            name: b"org.freedesktop.DBus.Error.NoMemory\0".as_ptr().cast(),
            message: std::ptr::null(),
            need_free: 0,
        };
    }
    -libc::ENOMEM
}

#[test]
fn error_uses_native_lengths_after_allocation_failure() {
    let name = Utf8CStr::from_bytes(b"org.example.Short\0").unwrap();
    let message = Utf8CStr::from_bytes(b"message that was not allocated\0").unwrap();
    let error = Error::new(name, Some(message));
    assert_eq!(
        error.name(),
        Utf8CStr::from_bytes(b"org.freedesktop.DBus.Error.NoMemory\0").unwrap()
    );
    assert_eq!(error.message(), None);
}
