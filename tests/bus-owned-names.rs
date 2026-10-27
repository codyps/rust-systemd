#![cfg(feature = "bus")]

use std::borrow::Cow;
use std::convert::TryFrom;
use std::ffi::{CStr, CString};
use systemd::bus::{
    Bus, BusName, BusNameBuf, InterfaceName, InterfaceNameBuf, MemberName, MemberNameBuf,
    ObjectPath, ObjectPathBuf,
};
use systemd::CStrArgument;

#[test]
fn constructors_borrow_or_transfer_checked_string_storage() {
    macro_rules! check {
        ($name:ty, $owned:ty, $valid:literal) => {{
            let input = CString::new($valid).unwrap();
            let borrowed = <$name>::new(input.as_c_str()).unwrap();
            assert!(matches!(borrowed, Cow::Borrowed(_)));
            assert_eq!(borrowed.as_ptr(), input.as_ptr());
            let owned: $owned = borrowed.into_owned();
            drop(input);
            assert_eq!(owned.to_bytes(), $valid.as_bytes());
            let checked = <$name>::new(&owned).unwrap();
            assert!(matches!(checked, Cow::Borrowed(_)));
            assert_eq!(checked.as_ptr(), owned.as_ptr());
            let input = CString::new($valid).unwrap();
            let original = input.as_ptr();
            let moved = <$name>::new(input).unwrap();
            assert!(matches!(moved, Cow::Owned(_)));
            assert_eq!(moved.as_ptr(), original);
            let moved = moved.try_into_cstr().unwrap();
            assert_eq!(moved.as_ptr(), original);
            let input = CString::new($valid).unwrap();
            let original = input.as_ptr();
            let owned = <$owned>::try_from(input).unwrap();
            let output = owned.into_c_string();
            assert_eq!(output.as_ptr(), original);
            let parsed: $owned = $valid.parse().unwrap();
            assert_eq!(parsed.to_bytes(), $valid.as_bytes());
            let converted = <$name>::new(String::from($valid)).unwrap();
            assert!(matches!(converted, Cow::Owned(_)));
            assert_eq!(converted.to_bytes(), $valid.as_bytes());
        }};
    }
    check!(ObjectPath, ObjectPathBuf, "/org/example/Stored");
    check!(BusName, BusNameBuf, "org.example.Stored");
    check!(InterfaceName, InterfaceNameBuf, "org.example.Stored");
    check!(MemberName, MemberNameBuf, "Stored");
    fn independent() -> Cow<'static, ObjectPath> {
        ObjectPath::new(String::from("/org/example/Independent")).unwrap()
    }
    assert_eq!(independent().to_bytes(), b"/org/example/Independent");
}

#[test]
fn constructors_reject_invalid_names_and_enforce_length_limits() {
    macro_rules! check {
        ($name:ty, $owned:ty, $invalid:literal) => {{
            for value in ["", $invalid, "interior\0nul", "two\0\0"] {
                assert_eq!(
                    <$name>::new(value).unwrap_err().kind(),
                    std::io::ErrorKind::InvalidInput
                );
                assert!(<$owned>::try_from(value.to_owned()).is_err());
            }
            let input = CString::new($invalid).unwrap();
            assert!(<$name>::new(input.as_c_str()).is_err());
            assert!(<$owned>::try_from(input).is_err());
            let bad_utf8 = CStr::from_bytes_with_nul(b"\xff\0").unwrap();
            assert!(<$name>::new(bad_utf8).is_err());
        }};
    }
    check!(ObjectPath, ObjectPathBuf, "/invalid path");
    check!(BusName, BusNameBuf, "invalid");
    check!(InterfaceName, InterfaceNameBuf, "invalid");
    check!(MemberName, MemberNameBuf, "Invalid.Member");
    let name = format!("a.{}", "b".repeat(253));
    assert!(InterfaceName::new(name.as_str()).is_ok());
    assert!(BusName::new(name.as_str()).is_ok());
    assert!(InterfaceName::new(format!("{}b", name)).is_err());
    assert!(BusName::new(format!("{}b", name)).is_err());
    assert!(MemberName::new("b".repeat(255)).is_ok());
    assert!(MemberName::new("b".repeat(256)).is_err());
}

#[test]
fn constructed_names_work_as_metadata_and_typed_message_values() {
    use foreign_types::ForeignType;
    let mut bus = Bus::default_system().unwrap();
    let mut message = bus
        .new_method_call(
            BusName::new("org.example.Stored").unwrap(),
            ObjectPath::new("/org/example/Stored").unwrap(),
            InterfaceName::new("org.example.Stored").unwrap(),
            MemberName::new("Stored").unwrap(),
        )
        .unwrap();
    assert_eq!(message.path().unwrap().to_bytes(), b"/org/example/Stored");
    message
        .append(ObjectPath::new("/org/example/Payload").unwrap())
        .unwrap();
    let stored = ObjectPath::new("/org/example/Array").unwrap().into_owned();
    message.append(vec![stored]).unwrap();
    message
        .append(BusName::new("org.example.Payload").unwrap())
        .unwrap();
    message
        .append(
            InterfaceName::new("org.example.Payload")
                .unwrap()
                .into_owned(),
        )
        .unwrap();
    message.append(MemberName::new("Payload").unwrap()).unwrap();
    message.append("invalid bus name").unwrap();
    assert_eq!(message.signature().to_bytes(), b"oaossss");
    // SAFETY: message is live and outgoing; sealing permits local reads.
    assert_eq!(
        unsafe { libsystemd_sys::bus::sd_bus_message_seal(message.as_ptr(), 1, 0) },
        0
    );
    let mut iter = message.iter().unwrap();
    let view: Cow<'_, ObjectPath> = iter.next().unwrap().unwrap();
    assert!(matches!(view, Cow::Borrowed(_)));
    let owned: Vec<ObjectPathBuf> = iter.next().unwrap().unwrap();
    let bus_name: BusNameBuf = iter.next().unwrap().unwrap();
    let interface: InterfaceNameBuf = iter.next().unwrap().unwrap();
    let member: MemberNameBuf = iter.next().unwrap().unwrap();
    assert_eq!(view.to_bytes(), b"/org/example/Payload");
    assert_eq!(
        iter.next::<BusNameBuf>().unwrap_err().kind(),
        std::io::ErrorKind::InvalidData
    );
    drop(view);
    drop(iter);
    drop(message);
    assert_eq!(owned[0].to_bytes(), b"/org/example/Array");
    assert_eq!(bus_name.to_bytes(), b"org.example.Payload");
    assert_eq!(interface.to_bytes(), b"org.example.Payload");
    assert_eq!(member.to_bytes(), b"Payload");
    let mut map = std::collections::HashMap::new();
    map.insert(owned[0].clone(), 42);
    let query = ObjectPath::from_bytes(b"/org/example/Array\0").unwrap();
    assert_eq!(map.get(query), Some(&42));
}
