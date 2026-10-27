use std::borrow::Cow;
use std::ffi::{CStr, CString};
use std::os::unix::io::AsRawFd;
use systemd::{daemon, id128::Id128, CStrArgument};

#[test]
fn c_strings_borrow_and_transfer_without_copying() {
    let value = CString::new("unchanged buffer").unwrap();
    let original = value.as_ptr();
    let borrowed = value.as_c_str().try_into_cstr().unwrap();
    assert!(matches!(borrowed, Cow::Borrowed(_)));
    assert_eq!(borrowed.as_ptr(), original);
    let borrowed = (&value).try_into_cstr().unwrap();
    assert!(matches!(borrowed, Cow::Borrowed(_)));
    assert_eq!(borrowed.as_ptr(), original);
    let moved = value.try_into_cstr().unwrap();
    assert!(matches!(moved, Cow::Owned(_)));
    assert_eq!(moved.as_ptr(), original);
    let moved_again = moved.try_into_cstr().unwrap();
    assert_eq!(moved_again.as_ptr(), original);
    let terminated = "already terminated\0";
    let borrowed = terminated.try_into_cstr().unwrap();
    assert!(matches!(borrowed, Cow::Borrowed(_)));
    assert_eq!(borrowed.as_ptr().cast::<u8>(), terminated.as_ptr());
}

#[test]
fn rust_strings_bytes_and_cows_preserve_content_and_reject_interior_nuls() {
    fn check<'a>(value: impl CStrArgument<'a>) {
        assert_eq!(
            value.try_into_cstr().unwrap().to_bytes_with_nul(),
            b"value\0"
        );
    }
    check("value");
    check("value\0");
    check(String::from("value"));
    check(String::from("value\0"));
    let string = String::from("value");
    let borrowed_string = &string;
    check(borrowed_string);
    check(&b"value"[..]);
    check(b"value\0".to_vec());
    let bytes = b"value".to_vec();
    let borrowed_bytes = &bytes;
    check(borrowed_bytes);
    check(Cow::<str>::Borrowed("value"));
    check(Cow::<str>::Owned(String::from("value")));
    check(Cow::<CStr>::Borrowed(
        CStr::from_bytes_with_nul(b"value\0").unwrap(),
    ));
    check(Cow::<CStr>::Owned(CString::new("value").unwrap()));
    check(CString::new("value").unwrap().into_boxed_c_str());
    for invalid in ["interior\0nul", "interior\0nul\0", "two\0\0"] {
        assert_eq!(
            invalid.try_into_cstr().unwrap_err().kind(),
            std::io::ErrorKind::InvalidInput
        );
        assert_eq!(
            invalid.to_owned().try_into_cstr().unwrap_err().kind(),
            std::io::ErrorKind::InvalidInput
        );
        assert_eq!(
            invalid
                .as_bytes()
                .to_vec()
                .try_into_cstr()
                .err()
                .unwrap()
                .kind(),
            std::io::ErrorKind::InvalidInput
        );
    }
    assert_eq!("".try_into_cstr().unwrap().to_bytes_with_nul(), b"\0");
    let bytes = CStr::from_bytes_with_nul(b"\xff\0").unwrap();
    assert_eq!(bytes.try_into_cstr().unwrap().as_ptr(), bytes.as_ptr());
}

#[test]
fn owned_daemon_paths_live_through_the_native_call() {
    let file = std::fs::File::open("/dev/null").unwrap();
    assert!(daemon::is_special(file.as_raw_fd(), Some(String::from("/dev/null"))).unwrap());
    assert!(
        daemon::is_special(file.as_raw_fd(), Some(CString::new("/dev/null").unwrap())).unwrap()
    );
    let path = CString::new("/dev/null").unwrap();
    assert!(daemon::is_special(file.as_raw_fd(), Some(path.as_c_str())).unwrap());
    assert!(daemon::is_special(
        file.as_raw_fd(),
        Some(Cow::<str>::Owned("/dev/null".to_owned()))
    )
    .unwrap());
    assert_eq!(
        daemon::is_special(file.as_raw_fd(), Some("bad\0path"))
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::InvalidInput
    );
}

#[test]
fn id_parsing_accepts_rust_and_c_strings() {
    let text = "0123456789abcdef0123456789abcdef";
    let expected = Id128::from_cstr(text).unwrap();
    assert_eq!(Id128::from_cstr(text.to_owned()).unwrap(), expected);
    let cstring = CString::new(text).unwrap();
    assert_eq!(Id128::from_cstr(&cstring).unwrap(), expected);
    assert_eq!(Id128::from_cstr(cstring.as_c_str()).unwrap(), expected);
    assert_eq!(Id128::from_cstr(cstring).unwrap(), expected);
    assert_eq!(
        Id128::from_cstr("bad\0id").unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );
}

#[test]
fn login_string_errors_are_consistent_without_panics() {
    let text = "rust-systemd-nonexistent-session";
    let expected = systemd::login::session_uid(text)
        .unwrap_err()
        .raw_os_error();
    assert_eq!(
        systemd::login::session_uid(text.to_owned())
            .unwrap_err()
            .raw_os_error(),
        expected
    );
    let cstring = CString::new(text).unwrap();
    assert_eq!(
        systemd::login::session_uid(cstring.as_c_str())
            .unwrap_err()
            .raw_os_error(),
        expected
    );
    assert_eq!(
        systemd::login::session_uid(cstring)
            .unwrap_err()
            .raw_os_error(),
        expected
    );
    assert_eq!(
        systemd::login::session_uid("bad\0session")
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::InvalidInput
    );
}

#[cfg(feature = "bus")]
#[test]
fn bus_metadata_containers_and_payloads_accept_mixed_string_forms() {
    use foreign_types::ForeignType;
    use systemd::bus::{Bus, ContainerType};
    let mut bus = Bus::default_system().unwrap();
    let mut message = bus
        .new_method_call(
            "org.example.Test",
            CString::new("/org/example/Test").unwrap(),
            CStr::from_bytes_with_nul(b"org.example.Test\0").unwrap(),
            String::from("Echo"),
        )
        .unwrap();
    assert_eq!(message.path().unwrap().to_bytes(), b"/org/example/Test");
    let mut signal = bus
        .new_signal(
            "/org/example/Test",
            "org.example.Test",
            CString::new("Changed").unwrap(),
        )
        .unwrap();
    signal
        .set_destination(Cow::<str>::Borrowed("org.example.Other"))
        .unwrap();
    assert_eq!(
        signal.destination().unwrap().to_bytes(),
        b"org.example.Other"
    );
    message
        .open_container(ContainerType::Array, CString::new("s").unwrap())
        .unwrap();
    message.append(CString::new("owned").unwrap()).unwrap();
    let borrowed = CString::new("borrowed").unwrap();
    message.append(borrowed.as_c_str()).unwrap();
    message.append(Cow::<str>::Owned("cow".to_owned())).unwrap();
    message.close_container().unwrap();
    let bad_utf8 = CStr::from_bytes_with_nul(b"\xff\0").unwrap();
    assert!(message.append(bad_utf8).is_err());
    // SAFETY: this is a live outgoing message; sealing permits local reads
    // without sending any test traffic to a service.
    assert_eq!(
        unsafe { libsystemd_sys::bus::sd_bus_message_seal(message.as_ptr(), 1, 0) },
        0
    );
    let mut iter = message.iter().unwrap();
    assert!(iter
        .enter_container(
            ContainerType::Array,
            CStr::from_bytes_with_nul(b"s\0").unwrap()
        )
        .unwrap());
    assert_eq!(
        iter.next::<CString>().unwrap().unwrap().to_bytes(),
        b"owned"
    );
    let view: &CStr = iter.next().unwrap().unwrap();
    assert_eq!(iter.next::<String>().unwrap().unwrap(), "cow");
    iter.exit_container().unwrap();
    assert_eq!(view.to_bytes(), b"borrowed");
    for (name, path, interface, member) in [
        ("invalid", "/org/example/Test", "org.example.Test", "Echo"),
        (
            "org.example.Test",
            "invalid path",
            "org.example.Test",
            "Echo",
        ),
        ("org.example.Test", "/org/example/Test", "invalid", "Echo"),
        (
            "org.example.Test",
            "/org/example/Test",
            "org.example.Test",
            "Bad.Member",
        ),
        (
            "org.example.Test",
            "/org/example/Test",
            "org.example.Test",
            "nul\0member",
        ),
    ] {
        assert_eq!(
            bus.new_method_call(name, path, interface, member)
                .err()
                .unwrap()
                .kind(),
            std::io::ErrorKind::InvalidInput
        );
    }
}

#[cfg(feature = "bus")]
#[test]
fn bus_errors_accept_strings_and_validate_utf8() {
    use systemd::bus::Error;
    let error = Error::new(
        CString::new("org.example.Failure").unwrap(),
        Some(String::from("details")),
    )
    .unwrap();
    assert_eq!(error.name().as_bytes(), b"org.example.Failure");
    assert_eq!(error.message().unwrap().as_bytes(), b"details");
    assert!(Error::new("org.example.NoMessage", None::<&str>)
        .unwrap()
        .message()
        .is_none());
    assert!(Error::new("invalid", None::<CString>).is_err());
    assert!(Error::new("org.example.Failure", Some("bad\0message")).is_err());
    let bad_utf8 = CStr::from_bytes_with_nul(b"\xff\0").unwrap();
    assert!(Error::new("org.example.Failure", Some(bad_utf8)).is_err());
}
