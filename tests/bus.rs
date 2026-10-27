#![cfg(feature = "bus")]

extern crate systemd;
extern crate utf8_cstr;

use systemd::bus;
use utf8_cstr::Utf8CStr;

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
