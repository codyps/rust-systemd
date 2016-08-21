use std::ptr;
use super::ffi::{c_char, pid_t};
use ffi::login as ffi;
use super::Result;
use mbox::MString;

/// Systemd slice and unit types
pub enum UnitType {
    /// User slice, service or scope unit
    UserUnit,
    /// System slice, service or scope unit
    SystemUnit,
}

/// Determines the systemd unit (i.e. service or scope unit) identifier of a process.
///
/// Specific processes can be optionally targeted via their PID. When no PID is
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

/// Determines the slice (either in system or user session) of a process.
///
/// Specific processes can be optionally targeted via their PID. When no PID is
/// specified, operation is executed for the calling process.
/// This method can be used to retrieve either a system or an user slice identifier.
pub fn get_slice(slice_type: UnitType, pid: Option<pid_t>) -> Result<String> {
    let mut c_slice_name: *mut c_char = ptr::null_mut();
    let p: pid_t = pid.unwrap_or(0);
    match slice_type {
        UnitType::UserUnit => sd_try!(ffi::sd_pid_get_user_slice(p, &mut c_slice_name)),
        UnitType::SystemUnit => sd_try!(ffi::sd_pid_get_slice(p, &mut c_slice_name))
    };
    let slice_id = unsafe { MString::from_raw(c_slice_name) };
    Ok(slice_id.unwrap().to_string())
}

/// Determines the machine name of a process.
///
/// Specific processes can be optionally targeted via their PID. When no PID is
/// specified, operation is executed for the calling process.
/// This method can be used to retrieve the machine name of processes running
/// inside a VM or a container.
pub fn get_machine_name(pid: Option<pid_t>) -> Result<String> {
    let mut c_machine_name: *mut c_char = ptr::null_mut();
    let p: pid_t = pid.unwrap_or(0);
    sd_try!(ffi::sd_pid_get_machine_name(p, &mut c_machine_name));
    let machine_id = unsafe { MString::from_raw(c_machine_name) };
    Ok(machine_id.unwrap().to_string())
}

/// Determines the control group path of a process.
///
/// Specific processes can be optionally targeted via their PID. When no PID is
/// specified, operation is executed for the calling process.
/// This method can be used to retrieve the control group path of a specific
/// process, relative to the root of the hierarchy. It returns the path without
/// trailing slash, except for processes located in the root control group,
/// where "/" is returned.
pub fn get_cgroup(pid: Option<pid_t>) -> Result<String> {
    let mut c_cgroup: *mut c_char = ptr::null_mut();
    let p: pid_t = pid.unwrap_or(0);
    sd_try!(ffi::sd_pid_get_cgroup(p, &mut c_cgroup));
    let cg = unsafe { MString::from_raw(c_cgroup) };
    Ok(cg.unwrap().to_string())
}
