#![cfg(feature = "bus")]

extern crate systemd;
extern crate utf8_cstr;

use utf8_cstr::Utf8CStr;
use systemd::bus;

#[test]
fn call() {
    let mut b = bus::Bus::default_system().unwrap();

    let mut m = b.new_method_call(
        bus::BusName::from_bytes(b"org.freedesktop.DBus\0").unwrap(),
        bus::ObjectPath::from_bytes(b"/\0").unwrap(),
        bus::InterfaceName::from_bytes(b"org.freedesktop.DBus\0").unwrap(),
        bus::MemberName::from_bytes(b"GetId\0").unwrap()
    ).unwrap();

    m.call(0).unwrap().unwrap();
}

#[test]
fn basic_append_and_read() {
    let mut b = bus::Bus::default_system().unwrap();

    let mut m = b.new_method_call(
        bus::BusName::from_bytes(b"org.freedesktop.DBus\0").unwrap(),
        bus::ObjectPath::from_bytes(b"/\0").unwrap(),
        bus::InterfaceName::from_bytes(b"org.freedesktop.DBus\0").unwrap(),
        bus::MemberName::from_bytes(b"GetNameOwner\0").unwrap()
    ).unwrap();

    m.append(Utf8CStr::from_bytes(b"org.freedesktop.DBus\0").unwrap()).unwrap();

    let mut r = m.call(0).unwrap().unwrap();

    let mut i = r.iter().unwrap();

    assert_eq!(i.peek_type().unwrap(), (b's' as ::std::os::raw::c_char, ""));

    let n : &Utf8CStr = i.next().unwrap().unwrap();
    assert_eq!(n, Utf8CStr::from_bytes(b"org.freedesktop.DBus\0").unwrap());
}
