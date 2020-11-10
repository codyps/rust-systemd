extern crate systemd;

use systemd::daemon::booted;
use systemd::login;

#[test]
fn test_get_unit() {
    let uu = login::get_unit(login::UnitType::UserUnit, None);
    let su = login::get_unit(login::UnitType::SystemUnit, None);
    let has_systemd = booted();
    assert!(has_systemd.is_ok());
    match has_systemd.unwrap() {
        // This is not running in a unit at all
        false => {
            assert!(uu.is_err());
            assert!(su.is_err());
        }
        // This is either running in a system or in a user unit
        true => {
            assert_eq!(uu.is_err(), su.is_ok());
        }
    };
}

#[test]
fn test_get_slice() {
    let us = login::get_slice(login::UnitType::UserUnit, None);
    let ss = login::get_slice(login::UnitType::SystemUnit, None);
    let has_systemd = booted();
    assert!(has_systemd.is_ok());
    match has_systemd.unwrap() {
        // This is running in the top-level generic slice
        false => {
            assert_eq!(ss.unwrap(), "-.slice");
        }
        // This is running in a system slice, and perhaps
        // in an user one too
        true => {
            if !ss.is_ok() && !us.is_ok() {
                panic!("ss: {:?}, us: {:?}", ss, us);
            }
        }
    };
}

#[test]
fn test_get_machine_name() {
    let mname = login::get_machine_name(None);
    let has_systemd = booted();
    assert!(has_systemd.is_ok());
    match has_systemd.unwrap() {
        // No machined registration
        false => {
            assert!(mname.is_err());
        }
        // This is unpredictable, based on testing environment
        true => {}
    };
}

#[test]
fn test_get_cgroup() {
    let cg = login::get_cgroup(None);
    let has_systemd = booted();
    assert!(has_systemd.is_ok());
    match has_systemd.unwrap() {
        // Running under systemd, inside a slice somewhere
        true => assert!(cg.is_ok()),
        // Nothing meaningful to check here
        false => {}
    };
}

#[test]
fn test_get_session() {
    let ss = login::get_session(None);
    let has_systemd = booted();
    assert!(has_systemd.is_ok());
    match has_systemd.unwrap() {
        // Running under systemd, inside a slice somewhere
        true => {
            ss.unwrap();
        }
        // Nothing meaningful to check here
        false => {}
    };
}

#[test]
fn test_get_owner_uid() {
    let ou = login::get_owner_uid(None);
    let has_systemd = booted();
    assert!(has_systemd.is_ok());
    match has_systemd.unwrap() {
        // Running under systemd, inside a slice somewhere
        true => {
            // even in this case, we might get a "no data avaliable" (github actions runners return
            // this)
            match ou {
                Err(e) => {
                    match e.raw_os_error() {
                        Some(libc::ENODATA) => { /* ok */ }
                        _ => panic!(e),
                    }
                }
                _ => { /* ok */ }
            }
        }
        // Nothing meaningful to check here
        false => {}
    };
}
