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
