#![feature(phase)]

#[phase(plugin,link)] extern crate systemd;

use systemd::daemon;

#[test]
fn test_listen_fds() {
    assert_eq!(daemon::listen_fds(false).unwrap(), 0);
}
