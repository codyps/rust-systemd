#![cfg(feature = "bus")]

use foreign_types::ForeignType;
use std::collections::{BTreeMap, HashMap};
use systemd::bus::types::{DictEntry, FromSdBusMessage, SdBusType, ToSdBusMessage, Variant};
use systemd::bus::{self, ContainerType, Message, MessageIter};
use utf8_cstr::Utf8CStr;

fn message() -> Message {
    bus::Bus::default_system()
        .unwrap()
        .new_method_call(
            bus::BusName::from_bytes(b"org.example.ContainerTest\0").unwrap(),
            bus::ObjectPath::from_bytes(b"/org/example/Test\0").unwrap(),
            bus::InterfaceName::from_bytes(b"org.example.Test\0").unwrap(),
            bus::MemberName::from_bytes(b"RoundTrip\0").unwrap(),
        )
        .unwrap()
}

fn seal(message: &mut Message) {
    // SAFETY: message owns a live outgoing message. Sealing it permits local
    // reads without sending test messages to any service.
    assert_eq!(
        unsafe { libsystemd_sys::bus::sd_bus_message_seal(message.as_ptr(), 1, 0) },
        0
    );
}

#[test]
fn nested_containers_round_trip() {
    let mut m = message();
    m.append(vec![vec![1u32, 2], vec![], vec![3]]).unwrap();
    m.append((7u32, "borrowed string", Variant(vec![(true, 42u64)])))
        .unwrap();
    m.append(Variant(Variant(vec![9i16, -2]))).unwrap();
    let dictionary: BTreeMap<_, _> = [
        ("first".to_owned(), Variant(vec![1u32, 2])),
        ("second".to_owned(), Variant(vec![])),
    ]
    .iter()
    .cloned()
    .collect();
    m.append(dictionary.clone()).unwrap();
    m.append("after containers").unwrap();
    assert_eq!(m.signature().to_bytes(), b"aau(usv)va{sv}s");
    seal(&mut m);
    let mut i = m.iter().unwrap();
    assert_eq!(
        i.next::<Vec<Vec<u32>>>().unwrap().unwrap(),
        vec![vec![1, 2], vec![], vec![3]]
    );
    let structure: (u32, &Utf8CStr, Variant<Vec<(bool, u64)>>) = i.next().unwrap().unwrap();
    assert_eq!(structure.0, 7);
    assert_eq!(structure.2, Variant(vec![(true, 42)]));
    assert_eq!(
        i.next::<Variant<Variant<Vec<i16>>>>().unwrap(),
        Some(Variant(Variant(vec![9, -2])))
    );
    assert_eq!(
        i.next::<BTreeMap<String, Variant<Vec<u32>>>>().unwrap(),
        Some(dictionary)
    );
    assert_eq!(i.next::<String>().unwrap().unwrap(), "after containers");
    assert_eq!(structure.1.as_bytes(), b"borrowed string");
    assert!(i.at_end().unwrap());
}

#[test]
fn empty_arrays_structs_and_maps_keep_their_types() {
    let mut m = message();
    m.append(Vec::<u32>::new()).unwrap();
    m.append(Vec::<Vec<String>>::new()).unwrap();
    m.append(BTreeMap::<String, u64>::new()).unwrap();
    m.append((Vec::<bool>::new(),)).unwrap();
    m.append(&[4u8, 5, 6][..]).unwrap();
    assert_eq!(m.signature().to_bytes(), b"auaasa{st}(ab)ay");
    seal(&mut m);
    let mut i = m.iter().unwrap();
    assert_eq!(i.next::<Vec<u32>>().unwrap(), Some(vec![]));
    assert_eq!(i.next::<Vec<Vec<String>>>().unwrap(), Some(vec![]));
    assert_eq!(
        i.next::<BTreeMap<String, u64>>().unwrap(),
        Some(BTreeMap::new())
    );
    assert_eq!(i.next::<(Vec<bool>,)>().unwrap(), Some((vec![],)));
    assert_eq!(i.next::<Vec<u8>>().unwrap(), Some(vec![4, 5, 6]));
    assert!(i.at_end().unwrap());
}

#[test]
fn dictionaries_support_borrowed_keys_hash_maps_and_duplicate_checks() {
    let mut m = message();
    m.append(vec![DictEntry("same", 1u32), DictEntry("same", 2)])
        .unwrap();
    m.append(vec![DictEntry("same", 1u32), DictEntry("same", 2)])
        .unwrap();
    m.append(vec![DictEntry("same", 1u32), DictEntry("same", 2)])
        .unwrap();
    let hash: HashMap<String, (u32, bool)> =
        [("one".to_owned(), (7, true))].iter().cloned().collect();
    m.append(hash.clone()).unwrap();
    seal(&mut m);
    let mut i = m.iter().unwrap();
    let entries: Vec<DictEntry<&Utf8CStr, u32>> = i.next().unwrap().unwrap();
    assert_eq!(entries[0].0.as_bytes(), b"same");
    assert_eq!((entries[0].1, entries[1].1), (1, 2));
    assert_eq!(
        i.next::<BTreeMap<String, u32>>().unwrap_err().kind(),
        std::io::ErrorKind::InvalidData
    );
    assert_eq!(
        i.next::<HashMap<String, u32>>().unwrap_err().kind(),
        std::io::ErrorKind::InvalidData
    );
    assert_eq!(
        i.next::<HashMap<String, (u32, bool)>>().unwrap(),
        Some(hash)
    );
    assert_eq!(entries[0].0.as_bytes(), b"same");
}

#[test]
fn type_mismatches_do_not_consume_containers() {
    let mut m = message();
    m.append(vec![7u32]).unwrap();
    m.append(Variant(42u32)).unwrap();
    m.append((1u32, "two")).unwrap();
    seal(&mut m);
    let mut i = m.iter().unwrap();
    assert_eq!(
        i.next::<Vec<u64>>().unwrap_err().raw_os_error(),
        Some(libc::ENXIO)
    );
    assert_eq!(i.next::<Vec<u32>>().unwrap(), Some(vec![7]));
    assert_eq!(
        i.next::<Variant<String>>().unwrap_err().raw_os_error(),
        Some(libc::ENXIO)
    );
    assert_eq!(i.next::<Variant<u32>>().unwrap(), Some(Variant(42)));
    assert_eq!(
        i.next::<(u32,)>().unwrap_err().raw_os_error(),
        Some(libc::ENXIO)
    );
    assert_eq!(
        i.next::<(u32, String)>().unwrap(),
        Some((1, "two".to_owned()))
    );
}

#[test]
fn native_array_encoding_is_readable_by_typed_decoder() {
    let mut m = message();
    // SAFETY: m is live and writable; "u" is a valid array element signature,
    // and each basic append receives a live u32 with the matching type.
    unsafe {
        assert_eq!(
            libsystemd_sys::bus::sd_bus_message_open_container(
                m.as_ptr(),
                b'a' as _,
                b"u\0".as_ptr().cast()
            ),
            0
        );
        for value in [11u32, 22, 33] {
            assert_eq!(
                libsystemd_sys::bus::sd_bus_message_append_basic(
                    m.as_ptr(),
                    b'u' as _,
                    (&value as *const u32).cast()
                ),
                0
            );
        }
        assert_eq!(
            libsystemd_sys::bus::sd_bus_message_close_container(m.as_ptr()),
            0
        );
    }
    seal(&mut m);
    assert_eq!(
        m.iter().unwrap().next::<Vec<u32>>().unwrap(),
        Some(vec![11, 22, 33])
    );
}

#[test]
fn cloned_message_cannot_invalidate_peeked_signature() {
    let mut m = message();
    m.append(vec![1u32]).unwrap();
    m.append(vec!["different signature"]).unwrap();
    seal(&mut m);
    let mut alias = m.clone();
    let mut first = m.iter().unwrap();
    let (_, signature) = first.peek_type().unwrap();
    let mut other = alias.iter().unwrap();
    assert_eq!(other.next::<Vec<u32>>().unwrap(), Some(vec![1]));
    assert_eq!(other.peek_type().unwrap().1, "s");
    other.next::<Vec<String>>().unwrap().unwrap();
    assert_eq!(signature, "u");
}

#[test]
fn invalid_signatures_and_out_of_context_entries_are_rejected() {
    struct Invalid;
    impl SdBusType for Invalid {
        fn signature() -> String {
            "u\0s".to_owned()
        }
    }
    impl ToSdBusMessage for Invalid {
        fn to_message(&self, _: &mut bus::MessageRef) -> systemd::Result<()> {
            panic!("invalid signature must be rejected before encoding elements")
        }
    }
    let mut m = message();
    assert_eq!(
        m.append(vec![Invalid]).unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );
    assert!(m.append(DictEntry(1u32, 2u32)).is_err());
    assert!(m.open_container(ContainerType::Array, "").is_err());
    assert!(m
        .open_container(ContainerType::Array, &format!("{}u", "a".repeat(65)))
        .is_err());
}

#[test]
fn failed_inner_decoder_restores_a_root_cursor() {
    struct Reject;
    impl SdBusType for Reject {
        fn signature() -> String {
            "u".to_owned()
        }
    }
    impl<'a> FromSdBusMessage<'a> for Reject {
        fn from_message(message: &mut MessageIter<'a>) -> systemd::Result<Option<Self>> {
            assert_eq!(message.next::<u32>()?, Some(7));
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "test decoder failure",
            ))
        }
    }
    let mut m = message();
    m.append(vec![7u32]).unwrap();
    seal(&mut m);
    let mut i = m.iter().unwrap();
    assert!(i.next::<Vec<Reject>>().is_err());
    assert_eq!(i.next::<Vec<u32>>().unwrap(), Some(vec![7]));
}

#[test]
fn basic_arrays_and_borrowed_paths_round_trip() {
    use std::os::unix::io::AsRawFd;
    let mut m = message();
    let path = bus::ObjectPath::from_bytes(b"/org/example/ArrayPath\0").unwrap();
    m.append(vec![true, false]).unwrap();
    m.append(vec![-1i16, 2]).unwrap();
    m.append(vec![1u16, 2]).unwrap();
    m.append(vec![-1i32, 2]).unwrap();
    m.append(vec![-1i64, 2]).unwrap();
    m.append(vec![1.5f64, -2.5]).unwrap();
    m.append(vec!["one", "two"]).unwrap();
    m.append(vec![path, path]).unwrap();
    let file = std::fs::File::open("/dev/null").unwrap();
    m.append(vec![bus::types::UnixFd(file.as_raw_fd())])
        .unwrap();
    assert_eq!(m.signature().to_bytes(), b"abanaqaiaxadasaoah");
    seal(&mut m);
    let mut i = m.iter().unwrap();
    assert_eq!(i.next::<Vec<bool>>().unwrap(), Some(vec![true, false]));
    assert_eq!(i.next::<Vec<i16>>().unwrap(), Some(vec![-1, 2]));
    assert_eq!(i.next::<Vec<u16>>().unwrap(), Some(vec![1, 2]));
    assert_eq!(i.next::<Vec<i32>>().unwrap(), Some(vec![-1, 2]));
    assert_eq!(i.next::<Vec<i64>>().unwrap(), Some(vec![-1, 2]));
    assert_eq!(i.next::<Vec<f64>>().unwrap(), Some(vec![1.5, -2.5]));
    let strings: Vec<&str> = i.next().unwrap().unwrap();
    let paths: Vec<&bus::ObjectPath> = i.next().unwrap().unwrap();
    let fds: Vec<bus::types::UnixFd> = i.next().unwrap().unwrap();
    assert_eq!(strings, ["one", "two"]);
    assert_eq!(paths[0].to_bytes_with_nul(), path.to_bytes_with_nul());
    // SAFETY: the decoded descriptor is still owned by the live message, and
    // F_GETFD inspects it without closing or otherwise modifying it.
    assert!(unsafe { libc::fcntl(fds[0].0, libc::F_GETFD) } >= 0);
}

#[test]
fn manual_container_traversal_checks_complete_consumption() {
    let mut m = message();
    m.open_container(ContainerType::Array, "u").unwrap();
    m.append(1u32).unwrap();
    m.append(2u32).unwrap();
    m.close_container().unwrap();
    m.append(3u32).unwrap();
    seal(&mut m);
    let mut i = m.iter().unwrap();
    assert!(i.enter_container(ContainerType::Array, "u").unwrap());
    assert_eq!(i.next::<u32>().unwrap(), Some(1));
    assert_eq!(
        i.exit_container().unwrap_err().raw_os_error(),
        Some(libc::EBUSY)
    );
    assert_eq!(i.next::<u32>().unwrap(), Some(2));
    assert!(i.at_end().unwrap());
    i.exit_container().unwrap();
    assert_eq!(i.next::<u32>().unwrap(), Some(3));
    i.rewind().unwrap();
    assert_eq!(i.next::<Vec<u32>>().unwrap(), Some(vec![1, 2]));
}

#[test]
fn panicking_decoder_unwinds_the_container_stack() {
    struct Panic;
    impl SdBusType for Panic {
        fn signature() -> String {
            "u".to_owned()
        }
    }
    impl<'a> FromSdBusMessage<'a> for Panic {
        fn from_message(message: &mut MessageIter<'a>) -> systemd::Result<Option<Self>> {
            assert_eq!(message.next::<u32>()?, Some(7));
            panic!("test decoder panic")
        }
    }
    let mut m = message();
    m.append(vec![vec![7u32]]).unwrap();
    seal(&mut m);
    let mut i = m.iter().unwrap();
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = i.next::<Vec<Vec<Panic>>>();
    }))
    .is_err());
    assert_eq!(i.next::<Vec<Vec<u32>>>().unwrap(), Some(vec![vec![7]]));
}
