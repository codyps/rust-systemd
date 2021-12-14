use std::ops::Deref;

use utf8_cstr::Utf8CStr;

// WARNING: you may want to use a more tested/complete dbus library, or one that is pure rust.
// `zbus` may be a reasonable choice, and there are others too

// approximately this command:
//     busctl --system call  org.freedesktop.systemd1 /org/freedesktop/systemd1 org.freedesktop.systemd1.Manager StartUnit "ss" "foo.service" "fail"
fn main() {
    let mut bus = systemd::bus::Bus::default_system().unwrap();

    let mut method_call = bus
        .new_method_call(
            systemd::bus::BusName::from_bytes(b"org.freedesktop.systemd1\0").unwrap(),
            systemd::bus::ObjectPath::from_bytes(b"/org/freedesktop/systemd1\0").unwrap(),
            systemd::bus::InterfaceName::from_bytes(b"org.freedesktop.systemd1.Manager\0").unwrap(),
            systemd::bus::MemberName::from_bytes(b"StartUnit\0").unwrap(),
        )
        .unwrap();

    // args
    method_call
        .append(Utf8CStr::from_bytes(b"foo.service\0").unwrap())
        .unwrap();
    method_call
        .append(Utf8CStr::from_bytes(b"fail\0").unwrap())
        .unwrap();

    let res = method_call.call(0).unwrap();

    eprintln!("done, result={:?}", res.deref());
}
