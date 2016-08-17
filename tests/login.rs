extern crate systemd;

use systemd::login;
use systemd::daemon::booted;

#[test]
fn test_get_unit() {
    let uu = login::get_unit(login::UnitType::UserUnit, None);
    let su = login::get_unit(login::UnitType::SystemUnit, None);
    let has_systemd = booted();
    if has_systemd.is_ok() {
        match has_systemd.unwrap() {
            // This is not running in an unit at all
            false => { assert!(uu.is_err()); assert!(su.is_err()); },
            // This is either running in a system or in an user unit
            true => { assert_eq!(uu.is_err(), su.is_ok()); },
        };
    }
}
