#![cfg(feature = "bus")]

use libsystemd_sys::bus::{sd_bus_error, sd_bus_error_set};
use systemd::bus::RawError;

#[test]
fn raw_error_ownership_transfer_and_copy() {
    let mut native = sd_bus_error {
        name: std::ptr::null(),
        message: std::ptr::null(),
        need_free: 0,
    };
    // SAFETY: native is initialized and empty; the literals are valid C strings.
    unsafe {
        sd_bus_error_set(
            &mut native,
            b"org.example.Ownership\0".as_ptr().cast(),
            b"native allocation\0".as_ptr().cast(),
        );
    }
    assert!(native.need_free > 0);
    // SAFETY: libsystemd initialized native's owned strings. Ownership moves
    // exclusively into this wrapper, which is responsible for freeing them.
    let error = unsafe { RawError::from_raw(native) };
    assert_eq!(error.name().unwrap().to_bytes(), b"org.example.Ownership");
    assert_eq!(error.message().unwrap().as_bytes(), b"native allocation");
    let copy = error.clone();
    drop(error);
    assert_eq!(copy.message().unwrap().as_bytes(), b"native allocation");
    drop(copy);

    // SAFETY: both strings are static and valid, and need_free=0 prevents the
    // wrapper from trying to release their non-owning storage.
    let borrowed = unsafe {
        RawError::from_raw(sd_bus_error {
            name: b"org.example.Static\0".as_ptr().cast(),
            message: b"static storage\0".as_ptr().cast(),
            need_free: 0,
        })
    };
    assert_eq!(borrowed.name().unwrap().to_bytes(), b"org.example.Static");
    assert_eq!(borrowed.message().unwrap().as_bytes(), b"static storage");
}
