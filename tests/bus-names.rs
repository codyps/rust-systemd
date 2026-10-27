#![cfg(feature = "bus")]

use systemd::bus::{BusName, InterfaceName, MemberName, ObjectPath};

#[test]
fn name_wrappers_validate_the_entire_c_string() {
    macro_rules! check {
        ($name:ty, $valid:expr) => {{
            let valid: &[u8] = $valid;
            let name = <$name>::from_bytes(valid).unwrap();
            assert_eq!(name.to_bytes_with_nul(), valid);
            assert!(<$name>::from_bytes(&valid[..valid.len() - 1]).is_err());
            for suffix in [&b"suffix"[..], &b"\0"[..], &b"suffix\0"[..], &b"\xff\0"[..]] {
                let mut invalid = valid.to_vec();
                invalid.extend_from_slice(suffix);
                assert!(<$name>::from_bytes(&invalid).is_err(), "{:?}", invalid);
            }
            // SAFETY: valid is a live, validated NUL-terminated name.
            let pointer_name = unsafe { <$name>::from_ptr_unchecked(valid.as_ptr().cast()) };
            assert_eq!(pointer_name.to_bytes(), &valid[..valid.len() - 1]);
            assert_eq!(pointer_name.to_bytes_with_nul(), valid);
        }};
    }
    check!(ObjectPath, b"/org/example/Complete\0");
    check!(ObjectPath, b"/\0");
    check!(BusName, b"org.example.Complete\0");
    check!(BusName, b":1.123\0");
    check!(InterfaceName, b"org.example.Complete\0");
    check!(MemberName, b"Complete\0");
}

#[test]
fn convertible_names_are_validated_at_each_public_bus_boundary() {
    use systemd::bus::{Bus, Vtable};
    let mut bus = Bus::default_system().unwrap();
    let invalid = std::io::ErrorKind::InvalidInput;
    assert_eq!(
        bus.request_name(String::from("invalid"), 0)
            .unwrap_err()
            .kind(),
        invalid
    );
    assert_eq!(bus.release_name("invalid").unwrap_err().kind(), invalid);
    assert_eq!(
        bus.request_name_async("invalid", 0, |_| Ok(()))
            .unwrap_err()
            .kind(),
        invalid
    );
    assert_eq!(
        bus.add_object("invalid", |_| Ok(())).unwrap_err().kind(),
        invalid
    );
    assert_eq!(
        bus.add_object_manager("invalid").unwrap_err().kind(),
        invalid
    );
    assert_eq!(
        bus.add_object_vtable("invalid", "org.example.Valid", Vtable::new())
            .unwrap_err()
            .kind(),
        invalid
    );
    assert_eq!(
        bus.add_object_vtable("/org/example/Valid", "invalid", Vtable::new())
            .unwrap_err()
            .kind(),
        invalid
    );
    for (path, interface, member) in [
        ("invalid", "org.example.Valid", "Valid"),
        ("/org/example/Valid", "invalid", "Valid"),
        ("/org/example/Valid", "org.example.Valid", "Invalid.Member"),
    ] {
        assert_eq!(
            bus.new_signal(path, interface, member)
                .err()
                .unwrap()
                .kind(),
            invalid
        );
    }
    let mut message = bus
        .new_signal("/org/example/Valid", "org.example.Valid", "Valid")
        .unwrap();
    assert_eq!(
        message.set_destination("invalid").unwrap_err().kind(),
        invalid
    );
    assert_eq!(message.send_to("invalid").unwrap_err().kind(), invalid);
    assert_eq!(
        message.send_to_no_reply("invalid").unwrap_err().kind(),
        invalid
    );
}
