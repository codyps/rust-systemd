#![cfg(feature = "bus")]

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use systemd::bus;
use utf8_cstr::Utf8CStr;

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
    let drops = Arc::new(AtomicUsize::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let canary = Canary(drops.clone());
    let observed = calls.clone();
    let path = bus::ObjectPath::from_bytes(b"/org/example/CompletePath\0").unwrap();
    server
        .add_object(path, move |message| {
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
        if case == "persistent" {
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
        "persistent",
        "call-complete",
        "call-drop",
        "call-failed",
        "name-complete",
        "name-drop",
        "name-failed",
    ] {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "callback_lifetimes_on_private_bus",
                "--nocapture",
            ])
            .env(CHILD_CASE, case)
            .env("DBUS_SYSTEM_BUS_ADDRESS", address.trim())
            .env("DBUS_SESSION_BUS_ADDRESS", address.trim())
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
