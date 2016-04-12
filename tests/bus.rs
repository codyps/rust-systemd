#![cfg(feature = "bus")]

extern crate systemd;

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
