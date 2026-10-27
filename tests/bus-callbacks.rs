#![cfg(feature = "bus")]

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use systemd::bus;
use utf8_cstr::Utf8CStr;

#[path = "bus-vtable/mod.rs"]
mod vtable;

struct Canary(Arc<AtomicUsize>);

impl Drop for Canary {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

fn pump(bus: &mut bus::BusRef) {
    while bus.process().unwrap().is_some() {}
}

fn drive(bus: &mut bus::BusRef, calls: &AtomicUsize) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while calls.load(Ordering::SeqCst) == 0 {
        pump(bus);
        assert!(Instant::now() < deadline, "callback timed out");
        if calls.load(Ordering::SeqCst) == 0 {
            bus.wait(Some(Duration::from_millis(10))).unwrap();
        }
    }
}

fn get_id(bus: &mut bus::BusRef) -> bus::Message {
    bus.new_method_call(
        bus::BusName::from_bytes(b"org.freedesktop.DBus\0").unwrap(),
        bus::ObjectPath::from_bytes(b"/\0").unwrap(),
        bus::InterfaceName::from_bytes(b"org.freedesktop.DBus\0").unwrap(),
        bus::MemberName::from_bytes(b"GetId\0").unwrap(),
    )
    .unwrap()
}

fn one_shot(case: &str) {
    let mut bus = bus::Bus::default_system().unwrap();
    let unique_name = bus.unique_name().unwrap();
    // SAFETY: libsystemd owns this live, NUL-terminated name for the bus lifetime.
    let native_name = unsafe { std::ffi::CStr::from_ptr(unique_name.as_ptr()) };
    assert_eq!(
        unique_name.to_bytes_with_nul(),
        native_name.to_bytes_with_nul()
    );
    let drops = Arc::new(AtomicUsize::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let canary = Canary(drops.clone());
    let observed = calls.clone();
    let callback = move |message: &mut bus::MessageRef| {
        let _keep_alive = &canary;
        assert!(message.error().is_none());
        observed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    };
    if case.starts_with("name-") {
        let name = bus::BusName::from_bytes(b"org.example.RustSystemdCallbackTest\0").unwrap();
        let flags = if case.ends_with("failed") {
            u64::MAX
        } else {
            0
        };
        let result = bus.request_name_async(name, flags, callback);
        assert_eq!(result.is_err(), case.ends_with("failed"));
    } else if case.ends_with("failed") {
        let mut signal = bus
            .new_signal(
                bus::ObjectPath::from_bytes(b"/org/example/Test\0").unwrap(),
                bus::InterfaceName::from_bytes(b"org.example.Test\0").unwrap(),
                bus::MemberName::from_bytes(b"Changed\0").unwrap(),
            )
            .unwrap();
        assert!(signal.call_async(callback, 1_000_000).is_err());
    } else {
        get_id(&mut bus).call_async(callback, 1_000_000).unwrap();
    }
    if case.ends_with("complete") {
        drive(&mut bus, &calls);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    } else if case.ends_with("failed") {
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    } else {
        assert_eq!(drops.load(Ordering::SeqCst), 0);
    }
    drop(bus);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

fn persistent_object() {
    let mut server = bus::Bus::default_system().unwrap();
    let mut client = bus::Bus::default_user().unwrap();
    client.unique_name().unwrap();
    server
        .request_name(String::from("org.example.ConvertibleName"), 0)
        .unwrap();
    server
        .release_name(std::ffi::CString::new("org.example.ConvertibleName").unwrap())
        .unwrap();
    let drops = Arc::new(AtomicUsize::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let canary = Canary(drops.clone());
    let observed = calls.clone();
    let path = bus::ObjectPath::from_bytes(b"/org/example/CompletePath\0").unwrap();
    let owned_path = std::ffi::CString::new(path.to_bytes()).unwrap();
    server
        .add_object(owned_path, move |message| {
            let _keep_alive = &canary;
            {
                let mut iter = message.iter().unwrap();
                let first: u32 = iter.next().unwrap().unwrap();
                let second: u32 = iter.next().unwrap().unwrap();
                let text: &Utf8CStr = iter.next().unwrap().unwrap();
                let decoded_path: &bus::ObjectPath = iter.next().unwrap().unwrap();
                let other_text: &Utf8CStr = iter.next().unwrap().unwrap();
                assert_eq!((first, second), (7, 42));
                assert_eq!(text.as_bytes(), b"first string");
                assert_eq!(other_text.as_bytes(), b"second string");
                assert_eq!(
                    decoded_path.to_bytes_with_nul(),
                    b"/org/example/CompletePath\0"
                );
                assert_eq!(iter.peek_type().unwrap().0, 0);
            }
            observed.fetch_add(1, Ordering::SeqCst);
            message
                .new_method_return()
                .unwrap()
                .send_no_reply()
                .unwrap();
            Ok(())
        })
        .unwrap();
    for expected in 1..=2 {
        let replies = Arc::new(AtomicUsize::new(0));
        let observed = replies.clone();
        {
            let mut call = client
                .new_method_call(
                    server.unique_name().unwrap(),
                    path,
                    bus::InterfaceName::from_bytes(b"org.example.Test\0").unwrap(),
                    bus::MemberName::from_bytes(b"Ping\0").unwrap(),
                )
                .unwrap();
            call.append(7u32).unwrap();
            call.append(42u32).unwrap();
            call.append(Utf8CStr::from_bytes(b"first string\0").unwrap())
                .unwrap();
            call.append(path).unwrap();
            call.append(Utf8CStr::from_bytes(b"second string\0").unwrap())
                .unwrap();
            call.call_async(
                move |reply| {
                    assert!(reply.error().is_none());
                    observed.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                },
                1_000_000,
            )
            .unwrap();
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        while replies.load(Ordering::SeqCst) == 0 {
            pump(&mut server);
            pump(&mut client);
            assert!(Instant::now() < deadline, "object reply timed out");
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(calls.load(Ordering::SeqCst), expected);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
    }
    drop(server);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

fn container_round_trip() {
    use bus::types::Variant;
    use std::collections::BTreeMap;
    let mut server = bus::Bus::default_system().unwrap();
    let mut client = bus::Bus::default_user().unwrap();
    client.unique_name().unwrap();
    let path = bus::ObjectPath::from_bytes(b"/org/example/Containers\0").unwrap();
    server
        .add_object(path, |request| {
            let dictionary: BTreeMap<String, Variant<Vec<u32>>> =
                request.iter().unwrap().next().unwrap().unwrap();
            assert_eq!(dictionary["numbers"], Variant(vec![1, 2, 3]));
            let mut reply = request.new_method_return().unwrap();
            reply.append(("received", dictionary)).unwrap();
            reply.send_no_reply().unwrap();
            Ok(())
        })
        .unwrap();
    let dictionary: BTreeMap<_, _> = [("numbers".to_owned(), Variant(vec![1u32, 2, 3]))]
        .iter()
        .cloned()
        .collect();
    let mut call = client
        .new_method_call(
            server.unique_name().unwrap(),
            path,
            bus::InterfaceName::from_bytes(b"org.example.Test\0").unwrap(),
            bus::MemberName::from_bytes(b"RoundTrip\0").unwrap(),
        )
        .unwrap();
    call.append(dictionary).unwrap();
    let replies = Arc::new(AtomicUsize::new(0));
    let observed = replies.clone();
    call.call_async(
        move |response| {
            assert!(response.error().is_none());
            let mut iter = response.iter().unwrap();
            let (text, dictionary): (&str, BTreeMap<String, Variant<Vec<u32>>>) =
                iter.next().unwrap().unwrap();
            assert_eq!(text, "received");
            assert_eq!(dictionary["numbers"], Variant(vec![1, 2, 3]));
            observed.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
        1_000_000,
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while replies.load(Ordering::SeqCst) == 0 {
        pump(&mut server);
        pump(&mut client);
        assert!(Instant::now() < deadline, "container reply timed out");
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn machine_connections() {
    use std::borrow::Cow;
    use std::ffi::{CStr, CString};

    // @.host selects this host and our current user. The subprocess points
    // its system-bus address at the private daemon, so no container or root
    // privileges are needed and the real host bus is never contacted.
    let default = bus::Bus::default_system().unwrap();
    let mut names = vec![default.unique_name().unwrap().to_bytes().to_vec()];
    fn check<'a>(machine: impl systemd::CStrArgument<'a>, names: &mut Vec<Vec<u8>>) {
        let mut connection = bus::Bus::open_system_machine(machine).unwrap();
        let name = connection.unique_name().unwrap().to_bytes().to_vec();
        assert!(!names.contains(&name), "connection must be independent");
        names.push(name);
        // Owned inputs have already been dropped; the connection must remain
        // usable, including the native authentication and method-call path.
        let mut reply = get_id(&mut connection).call(1_000_000).unwrap();
        let id: String = reply.iter().unwrap().next().unwrap().unwrap();
        assert!(!id.is_empty());
    }
    check("@.host", &mut names);
    check(String::from("@.host"), &mut names);
    let rust_string = String::from("@.host");
    check(&rust_string, &mut names);
    let c_string = CString::new("@.host").unwrap();
    check(c_string.as_c_str(), &mut names);
    check(&c_string, &mut names);
    check(c_string, &mut names);
    check(Cow::<str>::Borrowed("@.host"), &mut names);
    check(Cow::<str>::Owned(String::from("@.host")), &mut names);
    check(
        Cow::<CStr>::Owned(CString::new("@.host").unwrap()),
        &mut names,
    );
}

fn independent_connections() {
    let mut defaults = [
        bus::Bus::default().unwrap(),
        bus::Bus::default_system().unwrap(),
        bus::Bus::default_user().unwrap(),
    ];
    let mut names: Vec<_> = defaults
        .iter()
        .map(|bus| bus.unique_name().unwrap().to_bytes().to_vec())
        .collect();
    let opens = [bus::Bus::open, bus::Bus::open_system, bus::Bus::open_user];
    let scopes = [
        defaults[0].scope().unwrap().to_bytes().to_vec(),
        b"system".to_vec(),
        b"user".to_vec(),
    ];
    for (open, scope) in opens.iter().zip(&scopes) {
        for _ in 0..2 {
            let mut connection = open().unwrap();
            assert_eq!(connection.scope().unwrap().to_bytes(), scope);
            let name = connection.unique_name().unwrap().to_bytes().to_vec();
            assert!(!names.contains(&name), "connection must be independent");
            names.push(name);
            let mut reply = get_id(&mut connection).call(1_000_000).unwrap();
            let id: String = reply.iter().unwrap().next().unwrap().unwrap();
            assert!(!id.is_empty());
            connection.close();
        }
    }
    // Closing independent connections must leave the shared defaults usable.
    for default in &mut defaults {
        get_id(default).call(1_000_000).unwrap();
    }
}

fn connection_errors() {
    macro_rules! check {
        ($open:path, $native:path) => {{
            let mut native = std::ptr::null_mut();
            // SAFETY: native is a valid output. This subprocess has invalid
            // addresses for both local buses, so no host connection is made.
            let result = unsafe { $native(&mut native) };
            if result >= 0 {
                unsafe { libsystemd_sys::bus::sd_bus_unref(native) };
                panic!("invalid bus address unexpectedly succeeded");
            }
            assert_eq!($open().err().unwrap().raw_os_error(), Some(-result));
        }};
    }
    check!(bus::Bus::open, libsystemd_sys::bus::sd_bus_open);
    check!(
        bus::Bus::open_system,
        libsystemd_sys::bus::sd_bus_open_system
    );
    check!(bus::Bus::open_user, libsystemd_sys::bus::sd_bus_open_user);
}

fn flush_and_close() {
    let mut receiver = bus::Bus::open_user().unwrap();
    let destination = receiver.unique_name().unwrap().to_owned();
    let mut sender = bus::Bus::open_system().unwrap();
    {
        let mut signal = sender
            .new_signal("/org/example/Shutdown", "org.example.Shutdown", "Flushed")
            .unwrap();
        signal.set_destination(destination).unwrap();
        signal.append("before close").unwrap();
        signal.send_no_reply().unwrap();
    }
    let mut alias = sender.clone();
    sender.flush().unwrap();
    assert_eq!(sender.n_queued_write().unwrap(), 0);
    sender.close();
    sender.close(); // Closing an already closed connection is harmless.
    assert_eq!(alias.fd().unwrap_err().raw_os_error(), Some(libc::ENOTCONN));
    assert_eq!(
        alias.flush().unwrap_err().raw_os_error(),
        Some(libc::ENOTCONN)
    );

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(Some(mut message)) = receiver.process().unwrap() {
            if message.member().map(|name| name.to_bytes()) == Some(b"Flushed") {
                let text: String = message.iter().unwrap().next().unwrap().unwrap();
                assert_eq!(text, "before close");
                break;
            }
        }
        assert!(Instant::now() < deadline, "flushed message not received");
        receiver.wait(Some(Duration::from_millis(10))).unwrap();
    }
    receiver.close();
}

struct PrivateBus(Child);

impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn callback_lifetimes_on_private_bus() {
    const CHILD_CASE: &str = "RUST_SYSTEMD_BUS_CALLBACK_CASE";
    if let Ok(case) = std::env::var(CHILD_CASE) {
        if case == "connections" {
            independent_connections();
        } else if case == "connection-errors" {
            connection_errors();
        } else if case == "flush-close" {
            flush_and_close();
        } else if case == "machine" {
            machine_connections();
        } else if case == "vtable" {
            vtable::round_trip();
        } else if case == "vtable-errors" {
            vtable::registration_errors();
        } else if case == "containers" {
            container_round_trip();
        } else if case == "persistent" {
            persistent_object();
        } else {
            one_shot(&case);
        }
        return;
    }
    let child = Command::new("dbus-daemon")
        .args(["--session", "--nofork", "--print-address=1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("install dbus-daemon to run the private-bus integration tests");
    let mut bus = PrivateBus(child);
    let mut address = String::new();
    BufReader::new(bus.0.stdout.take().unwrap())
        .read_line(&mut address)
        .unwrap();
    assert!(!address.is_empty(), "private bus failed to start");
    for case in [
        "connections",
        "connection-errors",
        "flush-close",
        "machine",
        "vtable",
        "vtable-errors",
        "containers",
        "persistent",
        "call-complete",
        "call-drop",
        "call-failed",
        "name-complete",
        "name-drop",
        "name-failed",
    ] {
        let bus_address = if case == "connection-errors" {
            "rust-systemd-invalid:"
        } else {
            address.trim()
        };
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "callback_lifetimes_on_private_bus",
                "--nocapture",
            ])
            .env(CHILD_CASE, case)
            .env("DBUS_SYSTEM_BUS_ADDRESS", bus_address)
            .env("DBUS_SESSION_BUS_ADDRESS", bus_address)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut timed_out = false;
        while child.try_wait().unwrap().is_none() {
            if Instant::now() >= deadline {
                timed_out = true;
                child.kill().unwrap();
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        assert!(
            !timed_out && output.status.success(),
            "{case}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        println!("{case}: passed");
    }
}
