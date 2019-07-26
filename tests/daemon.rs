extern crate systemd;

use systemd::daemon;

#[test]
fn test_listen_fds() {
    assert_eq!(daemon::listen_fds(false).ok().unwrap(), 0);
}

#[test]
fn test_booted() {
    let result = daemon::booted();
    assert!(result.is_ok());
}

#[test]
fn test_watchdog_enabled() {
    let result = daemon::watchdog_enabled(false);
    assert!(result.is_ok());
    assert_eq!(result.ok().unwrap(), 0);
}

#[test]
fn test_notify() {
    let result = daemon::notify(
        false,
        [
            (daemon::STATE_READY, "1"),
            (daemon::STATE_STATUS, "Running test_notify()"),
        ]
        .into_iter(),
    );
    assert!(result.is_ok());
    assert_eq!(result.ok().unwrap(), false); // should fail, since this is not systemd-launched.
}
