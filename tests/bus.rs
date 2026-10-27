#![cfg(feature = "bus")]

extern crate systemd;
extern crate utf8_cstr;

use systemd::bus;
use utf8_cstr::Utf8CStr;

#[test]
fn finite_wait_timeouts_return_overflow_errors() {
    let mut bus = bus::Bus::default_system().unwrap();
    for timeout in [
        std::time::Duration::from_micros(u64::MAX),
        std::time::Duration::from_micros(u64::MAX) + std::time::Duration::from_micros(1),
        std::time::Duration::MAX,
    ] {
        assert_eq!(
            bus.wait(Some(timeout)).unwrap_err().raw_os_error(),
            Some(libc::EOVERFLOW)
        );
    }
    bus.wait(Some(std::time::Duration::ZERO)).unwrap();
}

#[test]
fn call() {
    let mut b = bus::Bus::default_system().unwrap();

    let mut m = b
        .new_method_call(
            bus::BusName::from_bytes(b"org.freedesktop.DBus\0").unwrap(),
            bus::ObjectPath::from_bytes(b"/\0").unwrap(),
            bus::InterfaceName::from_bytes(b"org.freedesktop.DBus\0").unwrap(),
            bus::MemberName::from_bytes(b"GetId\0").unwrap(),
        )
        .unwrap();

    assert!(m.error().is_none());
    let reply = m.call(0).unwrap();
    assert!(reply.error().is_none());
    assert!(reply.sender().is_some());
    assert_eq!(
        reply.destination().unwrap(),
        b.unique_name().unwrap().as_ref()
    );

    let name = Utf8CStr::from_bytes(b"org.example.TestError\0").unwrap();
    let message = Utf8CStr::from_bytes(b"A distinct error message\0").unwrap();
    for message in [Some(message), None] {
        let error = bus::Error::new(name, message).unwrap();
        let reply = m.new_method_error(&error).unwrap();
        let raw = reply.error().expect("error reply must contain its error");
        assert_eq!(
            raw.name().unwrap().as_ref().to_bytes(),
            b"org.example.TestError"
        );
        assert_eq!(raw.message(), message);
    }
}

#[test]
fn basic_append_and_read() {
    let mut b = bus::Bus::default_system().unwrap();

    let mut m = b
        .new_method_call(
            bus::BusName::from_bytes(b"org.freedesktop.DBus\0").unwrap(),
            bus::ObjectPath::from_bytes(b"/\0").unwrap(),
            bus::InterfaceName::from_bytes(b"org.freedesktop.DBus\0").unwrap(),
            bus::MemberName::from_bytes(b"GetNameOwner\0").unwrap(),
        )
        .unwrap();

    m.append(Utf8CStr::from_bytes(b"org.freedesktop.DBus\0").unwrap())
        .unwrap();

    let mut r = m.call(0).unwrap();

    let mut i = r.iter().unwrap();

    assert_eq!(i.peek_type().unwrap(), (b's' as ::std::os::raw::c_char, ""));

    let n: &Utf8CStr = i.next().unwrap().unwrap();
    assert_eq!(n, Utf8CStr::from_bytes(b"org.freedesktop.DBus\0").unwrap());
}

#[test]
fn bad_signature_on_call() {
    let mut b = bus::Bus::default_system().unwrap();

    let mut m = b
        .new_method_call(
            bus::BusName::from_bytes(b"org.freedesktop.DBus\0").unwrap(),
            bus::ObjectPath::from_bytes(b"/\0").unwrap(),
            bus::InterfaceName::from_bytes(b"org.freedesktop.DBus\0").unwrap(),
            bus::MemberName::from_bytes(b"GetNameOwner\0").unwrap(),
        )
        .unwrap();

    m.append(23u64).unwrap();

    let r = m.call(0).err().unwrap();
    println!("{r:?}");
}

#[test]
fn missing_headers_are_optional_and_debuggable() {
    let mut b = bus::Bus::default_system().unwrap();
    let path = bus::ObjectPath::from_bytes(b"/org/example/Test\0").unwrap();
    let interface = bus::InterfaceName::from_bytes(b"org.example.Test\0").unwrap();
    let member = bus::MemberName::from_bytes(b"Changed\0").unwrap();
    let signal = b.new_signal(path, interface, member).unwrap();
    assert_eq!(signal.sender(), None);
    assert_eq!(signal.destination(), None);
    assert!(format!("{:?}", *signal).contains("None"));

    let destination = bus::BusName::from_bytes(b"org.example.Test\0").unwrap();
    let call = b
        .new_method_call(destination, path, interface, member)
        .unwrap();
    assert_eq!(call.sender(), None);
    assert_eq!(call.destination().unwrap().to_bytes(), b"org.example.Test");
    assert!(format!("{:?}", *call).contains("sender: None"));
}

#[test]
fn machine_connection_errors_preserve_native_errors_and_reject_nuls() {
    use std::ffi::CString;

    for invalid in ["", "invalid/machine", "user@@machine"] {
        let c_string = CString::new(invalid).unwrap();
        let mut native = std::ptr::null_mut();
        // SAFETY: c_string is live and terminated; native is a valid output.
        let result = unsafe {
            libsystemd_sys::bus::sd_bus_open_system_machine(&mut native, c_string.as_ptr())
        };
        if result >= 0 {
            // Avoid leaking the native reference if the invalid fixture ever
            // becomes valid in a future libsystemd version.
            unsafe { libsystemd_sys::bus::sd_bus_unref(native) };
            panic!("invalid machine specification unexpectedly succeeded");
        }
        for error in [
            bus::Bus::open_system_machine(invalid).err().unwrap(),
            bus::Bus::open_system_machine(invalid.to_owned())
                .err()
                .unwrap(),
            bus::Bus::open_system_machine(c_string.as_c_str())
                .err()
                .unwrap(),
            bus::Bus::open_system_machine(c_string).err().unwrap(),
        ] {
            assert_eq!(error.raw_os_error(), Some(-result));
        }
    }
    for invalid in ["bad\0machine", "bad\0machine\0", "two\0\0"] {
        for error in [
            bus::Bus::open_system_machine(invalid).err().unwrap(),
            bus::Bus::open_system_machine(invalid.to_owned())
                .err()
                .unwrap(),
        ] {
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        }
    }
}

#[test]
fn remote_connection_errors_preserve_native_errors_and_reject_nuls() {
    use std::borrow::Cow;
    use std::ffi::{CStr, CString};
    use systemd::CStrArgument;

    // Native parsing rejects these before trying to launch SSH or connect.
    for invalid in ["[invalid", "user@", "user@@host", "host:123invalid/guest"] {
        let c_string = CString::new(invalid).unwrap();
        let mut native = std::ptr::null_mut();
        // SAFETY: c_string is live and terminated; native is a valid output.
        let result = unsafe {
            libsystemd_sys::bus::sd_bus_open_system_remote(&mut native, c_string.as_ptr())
        };
        if result >= 0 {
            unsafe {
                libsystemd_sys::bus::sd_bus_close(native);
                libsystemd_sys::bus::sd_bus_unref(native);
            }
            panic!("invalid remote specification unexpectedly succeeded");
        }
        fn check<'a>(host: impl CStrArgument<'a>, expected: i32) {
            let error = bus::Bus::open_system_remote(host).err().unwrap();
            assert_eq!(error.raw_os_error(), Some(expected));
        }
        check(invalid, -result);
        check(invalid.to_owned(), -result);
        check(c_string.as_c_str(), -result);
        check(&c_string, -result);
        check(Cow::<str>::Borrowed(invalid), -result);
        check(Cow::<str>::Owned(invalid.to_owned()), -result);
        check(Cow::<CStr>::Borrowed(c_string.as_c_str()), -result);
        check(Cow::<CStr>::Owned(c_string.clone()), -result);
        check(c_string, -result);
    }
    for invalid in ["bad\0host", "bad\0host\0", "two\0\0"] {
        for error in [
            bus::Bus::open_system_remote(invalid).err().unwrap(),
            bus::Bus::open_system_remote(invalid.to_owned())
                .err()
                .unwrap(),
        ] {
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        }
    }
}
