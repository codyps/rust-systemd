extern crate systemd;

use systemd::daemon;

#[test]
fn test_listen_fds() {
    assert_eq!(daemon::listen_fds(false).unwrap(), 0);
}

#[test]
fn test_booted() {
    let result = daemon::booted();
    assert!(result.is_ok());
    // Assuming that anyone using this library is probably running systemd. Is
    // that correct?
    assert!(result.unwrap());
}

#[test]
fn test_watchdog_enabled() {
    let result = daemon::watchdog_enabled(false);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_notify() {
    let mut state = ::std::collections::HashMap::new();
    state.insert(daemon::STATE_READY, "1");
    state.insert(daemon::STATE_STATUS, "Running test_notify()");
    let result = daemon::notify(false, state);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), false); // should fail, since this is not systemd-launched.
}
