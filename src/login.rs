use std::ptr;
use super::ffi::{c_char, pid_t};
use ffi::login as ffi;
use super::Result;
use mbox::MString;

/// Systemd unit types
pub enum UnitType {
    /// User service or scope unit
    UserUnit,
    /// System service or scope unit
    SystemUnit,
}

/// Determines the systemd unit (i.e. service or scope unit) identifier of a process.
///
/// Specific processess can be targeted by optionally via PID. When no PID is
/// specified, operation is executed for the calling process.
/// This method can be used to retrieve either a system or an user unit identifier.
pub fn get_unit(unit_type: UnitType, pid: Option<pid_t>) -> Result<String> {
    let mut c_unit_name: *mut c_char = ptr::null_mut();
    let p: pid_t = pid.unwrap_or(0);
    match unit_type {
        UnitType::UserUnit => sd_try!(ffi::sd_pid_get_user_unit(p, &mut c_unit_name)),
        UnitType::SystemUnit => sd_try!(ffi::sd_pid_get_unit(p, &mut c_unit_name))
    };
    let unit_name = unsafe { MString::from_raw(c_unit_name) };
    Ok(unit_name.unwrap().to_string())
}
