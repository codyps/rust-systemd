use super::{pump, Canary};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use systemd::bus::types::Variant;
use systemd::bus::{self, MessageRef, Vtable, VtableFlags as Flags};
use utf8_cstr::Utf8CStr;

fn path() -> &'static bus::ObjectPath {
    bus::ObjectPath::from_bytes(b"/org/example/Vtable\0").unwrap()
}
fn interface() -> &'static bus::InterfaceName {
    bus::InterfaceName::from_bytes(b"org.example.Vtable\0").unwrap()
}

fn exchange(
    server: &mut bus::BusRef,
    client: &mut bus::BusRef,
    interface: &'static [u8],
    member: &'static [u8],
    body: impl FnOnce(&mut MessageRef),
    check: impl Fn(&mut MessageRef) + Send + Sync + 'static,
) {
    let mut call = client
        .new_method_call(
            server.unique_name().unwrap(),
            path(),
            bus::InterfaceName::from_bytes(interface).unwrap(),
            bus::MemberName::from_bytes(member).unwrap(),
        )
        .unwrap();
    body(&mut call);
    let replies = Arc::new(AtomicUsize::new(0));
    let observed = replies.clone();
    call.call_async(
        move |reply| {
            check(reply);
            observed.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
        1_000_000,
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while replies.load(Ordering::SeqCst) == 0 {
        pump(server);
        pump(client);
        assert!(Instant::now() < deadline, "vtable reply timed out");
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(replies.load(Ordering::SeqCst), 1);
}

fn expect_error(reply: &mut MessageRef) {
    assert!(
        reply.error().is_some(),
        "expected an error reply: {:?}",
        reply
    );
}

pub fn round_trip() {
    let mut server = bus::Bus::default_system().unwrap();
    let mut client = bus::Bus::default_user().unwrap();
    client.unique_name().unwrap();
    let count = Arc::new(AtomicU32::new(1));
    let get = count.clone();
    let set = count.clone();
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let drops = Arc::new(AtomicUsize::new(0));
    let canary = Canary(drops.clone());
    // All definition strings and callback captures are heap-owned after this
    // block, and remain live while processing and unregistering the vtable.
    {
        let echo = String::from("Echo");
        let signature = String::from("a{sv}");
        let table = Vtable::new()
            .method(
                &echo,
                &signature,
                &signature,
                Flags::UNPRIVILEGED,
                move |request| {
                    let _keep_alive = &canary;
                    let dictionary: BTreeMap<String, Variant<Vec<u32>>> =
                        request.iter()?.next()?.unwrap();
                    let mut reply = request.new_method_return()?;
                    reply.append(dictionary)?;
                    reply.send_no_reply()?;
                    observed.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                },
            )
            .unwrap()
            .method("Fail", "", "", Flags::UNPRIVILEGED, |_| {
                Err(bus::Error::new(
                    Utf8CStr::from_bytes(b"org.example.VtableFailure\0").unwrap(),
                    Some(Utf8CStr::from_bytes(b"deliberate failure\0").unwrap()),
                ))
            })
            .unwrap()
            .method("Panic", "", "", Flags::UNPRIVILEGED, |_| {
                panic!("callback test panic")
            })
            .unwrap()
            .method("WrongReply", "", "u", Flags::UNPRIVILEGED, |request| {
                let mut reply = request.new_method_return()?;
                reply.append("wrong type")?;
                reply.send_no_reply()?;
                Ok(())
            })
            .unwrap()
            .property("ReadOnly", "u", Flags::PROPERTY_CONST, |reply| {
                reply.append(11u32)?;
                Ok(())
            })
            .unwrap()
            .property("Broken", "u", Flags::PROPERTY_EXPLICIT, |_| {
                Err(std::io::Error::from_raw_os_error(libc::EACCES).into())
            })
            .unwrap()
            .property("PanicProperty", "u", Flags::PROPERTY_EXPLICIT, |_| {
                panic!("getter test panic")
            })
            .unwrap()
            .writable_property(
                "Count",
                "u",
                Flags::UNPRIVILEGED,
                move |reply| {
                    reply.append(get.load(Ordering::SeqCst))?;
                    Ok(())
                },
                move |value| {
                    let value = value.next::<u32>()?.unwrap();
                    if value == 13 {
                        return Err(std::io::Error::from_raw_os_error(libc::EINVAL).into());
                    }
                    assert_ne!(value, 14, "setter test panic");
                    set.store(value, Ordering::SeqCst);
                    Ok(())
                },
            )
            .unwrap()
            .signal("Changed", "u", Flags::empty())
            .unwrap();
        server
            .add_object_vtable(path(), interface(), table)
            .unwrap();
    }
    for _ in 0..2 {
        exchange(
            &mut server,
            &mut client,
            b"org.example.Vtable\0",
            b"Echo\0",
            |call| {
                let dictionary: BTreeMap<_, _> =
                    [("numbers".to_owned(), Variant(vec![1u32, 2, 3]))]
                        .iter()
                        .cloned()
                        .collect();
                call.append(dictionary).unwrap();
            },
            |reply| {
                assert!(reply.error().is_none());
                assert_eq!(reply.signature().to_bytes(), b"a{sv}");
                let dictionary: BTreeMap<String, Variant<Vec<u32>>> =
                    reply.iter().unwrap().next().unwrap().unwrap();
                assert_eq!(dictionary["numbers"], Variant(vec![1, 2, 3]));
            },
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    exchange(
        &mut server,
        &mut client,
        b"org.example.Vtable\0",
        b"Echo\0",
        |call| call.append("wrong signature").unwrap(),
        expect_error,
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    exchange(
        &mut server,
        &mut client,
        b"org.example.Vtable\0",
        b"Fail\0",
        |_| {},
        |reply| {
            let error = reply.error().unwrap();
            assert_eq!(
                error.name().unwrap().to_bytes(),
                b"org.example.VtableFailure"
            );
            assert_eq!(error.message().unwrap().as_bytes(), b"deliberate failure");
        },
    );
    for member in [b"Panic\0".as_slice(), b"WrongReply\0", b"Missing\0"] {
        exchange(
            &mut server,
            &mut client,
            b"org.example.Vtable\0",
            member,
            |_| {},
            expect_error,
        );
    }
    exchange(
        &mut server,
        &mut client,
        b"org.freedesktop.DBus.Properties\0",
        b"Set\0",
        |call| {
            call.append("org.example.Vtable").unwrap();
            call.append("Count").unwrap();
            call.append(Variant(42u32)).unwrap();
        },
        |reply| {
            assert!(reply.error().is_none());
        },
    );
    assert_eq!(count.load(Ordering::SeqCst), 42);
    for value in [13u32, 14] {
        exchange(
            &mut server,
            &mut client,
            b"org.freedesktop.DBus.Properties\0",
            b"Set\0",
            move |call| {
                call.append("org.example.Vtable").unwrap();
                call.append("Count").unwrap();
                call.append(Variant(value)).unwrap();
            },
            expect_error,
        );
    }
    assert_eq!(count.load(Ordering::SeqCst), 42);
    for property in ["ReadOnly", "Count"] {
        exchange(
            &mut server,
            &mut client,
            b"org.freedesktop.DBus.Properties\0",
            b"Set\0",
            move |call| {
                call.append("org.example.Vtable").unwrap();
                call.append(property).unwrap();
                call.append(Variant("wrong type")).unwrap();
            },
            expect_error,
        );
    }
    assert_eq!(count.load(Ordering::SeqCst), 42);
    exchange(
        &mut server,
        &mut client,
        b"org.freedesktop.DBus.Properties\0",
        b"Get\0",
        |call| {
            call.append("org.example.Vtable").unwrap();
            call.append("Count").unwrap();
        },
        |reply| {
            assert!(reply.error().is_none());
            assert_eq!(
                reply.iter().unwrap().next::<Variant<u32>>().unwrap(),
                Some(Variant(42))
            );
        },
    );
    for property in ["Broken", "PanicProperty"] {
        exchange(
            &mut server,
            &mut client,
            b"org.freedesktop.DBus.Properties\0",
            b"Get\0",
            move |call| {
                call.append("org.example.Vtable").unwrap();
                call.append(property).unwrap();
            },
            expect_error,
        );
    }
    exchange(
        &mut server,
        &mut client,
        b"org.freedesktop.DBus.Properties\0",
        b"GetAll\0",
        |call| call.append("org.example.Vtable").unwrap(),
        |reply| {
            assert!(reply.error().is_none(), "{:?}", reply);
            let values: BTreeMap<String, Variant<u32>> =
                reply.iter().unwrap().next().unwrap().unwrap();
            assert_eq!(values.len(), 2);
            assert_eq!(values["ReadOnly"], Variant(11));
            assert_eq!(values["Count"], Variant(42));
        },
    );
    exchange(
        &mut server,
        &mut client,
        b"org.freedesktop.DBus.Introspectable\0",
        b"Introspect\0",
        |_| {},
        |reply| {
            assert!(reply.error().is_none());
            let xml: String = reply.iter().unwrap().next().unwrap().unwrap();
            for expected in [
                "name=\"org.example.Vtable\"",
                "name=\"Echo\"",
                "type=\"a{sv}\"",
                "name=\"Count\"",
                "access=\"readwrite\"",
                "name=\"ReadOnly\"",
                "name=\"Changed\"",
            ] {
                assert!(xml.contains(expected), "missing {}: {}", expected, xml);
            }
        },
    );
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(server);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

pub fn registration_errors() {
    let mut server = bus::Bus::default_system().unwrap();
    let mut client = bus::Bus::default_user().unwrap();
    client.unique_name().unwrap();
    let drops = Arc::new(AtomicUsize::new(0));
    let handler = || {
        let canary = Canary(drops.clone());
        move |_: &mut MessageRef| {
            let _keep_alive = &canary;
            Ok(())
        }
    };
    assert!(Vtable::new()
        .method("Bad.Name", "", "", Flags::empty(), handler())
        .is_err());
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(Vtable::new()
        .method("Nul", "s\0u", "", Flags::empty(), handler())
        .is_err());
    assert_eq!(drops.load(Ordering::SeqCst), 2);
    let table = Vtable::new()
        .method("Invalid", "(", "", Flags::empty(), handler())
        .unwrap();
    assert!(server
        .add_object_vtable(path(), interface(), table)
        .is_err());
    assert_eq!(drops.load(Ordering::SeqCst), 3);
    let table = Vtable::new()
        .method("Existing", "", "", Flags::UNPRIVILEGED, |request| {
            request.new_method_return()?.send_no_reply()?;
            Ok(())
        })
        .unwrap();
    server
        .add_object_vtable(path(), interface(), table)
        .unwrap();
    let table = Vtable::new()
        .method("Partial", "", "", Flags::UNPRIVILEGED, handler())
        .unwrap()
        .method("Existing", "", "", Flags::UNPRIVILEGED, handler())
        .unwrap();
    assert!(server
        .add_object_vtable(path(), interface(), table)
        .is_err());
    assert_eq!(drops.load(Ordering::SeqCst), 5);
    exchange(
        &mut server,
        &mut client,
        b"org.example.Vtable\0",
        b"Existing\0",
        |_| {},
        |reply| assert!(reply.error().is_none()),
    );
    exchange(
        &mut server,
        &mut client,
        b"org.example.Vtable\0",
        b"Partial\0",
        |_| {},
        expect_error,
    );
    let table = Vtable::new()
        .writable_property("Bad", "u", Flags::PROPERTY_CONST, handler(), |_| Ok(()))
        .unwrap();
    assert!(server
        .add_object_vtable(path(), interface(), table)
        .is_err());
    assert_eq!(drops.load(Ordering::SeqCst), 6);
}
