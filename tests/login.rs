extern crate systemd;

use systemd::daemon::booted;
use systemd::login;

#[test]
fn test_get_unit() {
    let uu = login::get_unit(login::UnitType::UserUnit, None);
    let su = login::get_unit(login::UnitType::SystemUnit, None);
    let has_systemd = booted();
    assert!(has_systemd.is_ok());
    if has_systemd.unwrap() {
        // User units run under a system unit (E.g. user@1000.service)
        assert!(su.is_ok());
    } else {
        // This is not running in a unit at all
        assert!(uu.is_err());
        assert!(su.is_err());
    }
}

#[test]
fn test_get_slice() {
    let us = login::get_slice(login::UnitType::UserUnit, None);
    let ss = login::get_slice(login::UnitType::SystemUnit, None);
    let has_systemd = booted();
    assert!(has_systemd.is_ok());
    if has_systemd.unwrap() {
        // This is running in a system slice, and perhaps
        // in an user one too
        if ss.is_err() && us.is_err() {
            panic!("ss: {:?}, us: {:?}", ss, us);
        }
    } else {
        // This is running in the top-level generic slice
        assert_eq!(ss.unwrap(), "-.slice");
    }
}

#[test]
fn test_get_machine_name() {
    let mname = login::get_machine_name(None);
    let has_systemd = booted();
    assert!(has_systemd.is_ok());
    if !has_systemd.unwrap() {
        // No machined registration
        assert!(mname.is_err());
    } else {
        // This is unpredictable, based on testing environment
    }
}

#[test]
fn test_get_cgroup() {
    let cg = login::get_cgroup(None);
    let has_systemd = booted();
    assert!(has_systemd.is_ok());
    if has_systemd.unwrap() {
        // Running under systemd, inside a slice somewhere
        assert!(cg.is_ok());
    } else {
        // Nothing meaningful to check here
    }
}

#[test]
fn test_get_session() {
    let ss = login::get_session(None);
    let has_systemd = booted();
    assert!(has_systemd.is_ok());
    if has_systemd.unwrap() {
        // Running under systemd, inside a slice somewhere
        // even in this case, we might get a "no data available" (github actions runners return
        // this)
        if let Err(e) = ss {
            match e.raw_os_error() {
                Some(libc::ENODATA) => { /* ok */ }
                _ => panic!("{}", e),
            }
        }
    } else {
        // Nothing meaningful to check here
    }
}

#[test]
fn test_get_owner_uid() {
    let ou = login::get_owner_uid(None);
    let has_systemd = booted();
    assert!(has_systemd.is_ok());
    if has_systemd.unwrap() {
        // Running under systemd, inside a slice somewhere
        // even in this case, we might get a "no data available" (github actions runners return
        // this)
        if let Err(e) = ou {
            match e.raw_os_error() {
                Some(libc::ENODATA) => { /* ok */ }
                _ => panic!("{}", e),
            }
        }
    } else {
        // Nothing meaningful to check here
    }
}

#[test]
fn test_get_sessions() {
    let sessions = login::get_sessions();
    let has_systemd = booted();
    assert!(has_systemd.is_ok());
    if has_systemd.unwrap() {
        // Running under systemd - sessions should be retrievable
        assert!(sessions.is_ok());
        let session_list = sessions.unwrap();
        // We can't assume any particular number of sessions, but the call should succeed
        println!("Found {} sessions", session_list.len());
    } else {
        // Without systemd, this should fail gracefully
        assert!(sessions.is_err());
    }
}

#[test]
fn test_session_functions() {
    let has_systemd = booted();
    assert!(has_systemd.is_ok());
    if !has_systemd.unwrap() {
        // Skip session-specific tests without systemd
        return;
    }

    let sessions = login::get_sessions();
    if sessions.is_err() {
        // No sessions available, skip session-specific tests
        return;
    }

    let session_list = sessions.unwrap();
    if session_list.is_empty() {
        // No sessions available, skip session-specific tests
        return;
    }

    // Test session functions with the first available session
    let session_id = &session_list[0];

    // Test get_session_uid
    let uid_result = login::get_session_uid(session_id);
    // UID should be retrievable for any valid session
    assert!(
        uid_result.is_ok(),
        "Failed to get UID for session {}: {:?}",
        session_id,
        uid_result
    );

    // Test get_session_start_time
    let start_time_result = login::get_session_start_time(session_id);
    // Start time should be retrievable for any valid session
    assert!(
        start_time_result.is_ok(),
        "Failed to get start time for session {}: {:?}",
        session_id,
        start_time_result
    );
    let start_time = start_time_result.unwrap();
    // Start time should be a reasonable timestamp (after year 2000)
    assert!(
        start_time > 946_684_800_000_000,
        "Start time seems too early: {}",
        start_time
    );

    // Test get_session_tty (may return None for non-TTY sessions)
    let tty_result = login::get_session_tty(session_id);
    assert!(
        tty_result.is_ok(),
        "Failed to get TTY for session {}: {:?}",
        session_id,
        tty_result
    );

    // Test get_session_remote_host (may return None for local sessions)
    let remote_host_result = login::get_session_remote_host(session_id);
    assert!(
        remote_host_result.is_ok(),
        "Failed to get remote host for session {}: {:?}",
        session_id,
        remote_host_result
    );

    // Test get_session_display (may return None for non-GUI sessions)
    let display_result = login::get_session_display(session_id);
    assert!(
        display_result.is_ok(),
        "Failed to get display for session {}: {:?}",
        session_id,
        display_result
    );

    // Test get_session_type (should return Some value for any session)
    let type_result = login::get_session_type(session_id);
    assert!(
        type_result.is_ok(),
        "Failed to get type for session {}: {:?}",
        session_id,
        type_result
    );
}
