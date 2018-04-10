use std::ptr;
use super::ffi::{c_char, c_uint, pid_t, uid_t};
use ffi::login as ffi;
use super::{Result, free_cstring};
use cstr_argument::CStrArgument;

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
    let unit_name = free_cstring(c_unit_name).unwrap();
    Ok(unit_name)
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
    let slice_id = free_cstring(c_slice_name).unwrap();
    Ok(slice_id)
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
    let machine_id = free_cstring(c_machine_name).unwrap();
    Ok(machine_id)
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
    let cg = free_cstring(c_cgroup).unwrap();
    Ok(cg)
}

/// Determines the session identifier of a process.
///
/// Specific processes can be optionally targeted via their PID. When no PID is
/// specified, operation is executed for the calling process.
/// This method can be used to retrieve a session identifier.
pub fn get_session(pid: Option<pid_t>) -> Result<String> {
    let mut c_session: *mut c_char = ptr::null_mut();
    let p: pid_t = pid.unwrap_or(0);
    sd_try!(ffi::sd_pid_get_session(p, &mut c_session));
    let ss = free_cstring(c_session).unwrap();
    Ok(ss)
}

/// Determines the seat identifier of the seat the session identified
/// by the specified session identifier belongs to.
///
/// Note that not all sessions are attached to a seat, this call will fail for them.
pub fn get_seat<S: CStrArgument>(session: S) -> Result<String> {
    let session = session.into_cstr();
    let mut c_seat: *mut c_char = ptr::null_mut();
    sd_try!(ffi::sd_session_get_seat(session.as_ref().as_ptr(), &mut c_seat));
    let ss = free_cstring(c_seat).unwrap();
    Ok(ss)
}

/// Determines the VT number of the session identified by the specified session identifier.
///
/// This function will return an error if the seat does not support VTs.
pub fn get_vt<S: CStrArgument>(session: S) -> Result<u32> {
    let session = session.into_cstr();
    let c_vt: *mut c_uint = ptr::null_mut();
    sd_try!(ffi::sd_session_get_vt(session.as_ref().as_ptr(), c_vt));
    Ok(unsafe { *c_vt })
}

/// Determines the owner uid of a process.
///
/// Specific processes can be optionally targeted via their PID. When no PID is
/// specified, operation is executed for the calling process.
/// This method can be used to retrieve an owner uid.
pub fn get_owner_uid(pid: Option<pid_t>) -> Result<uid_t> {
    let mut c_owner_uid: u32 = 0u32;
    let p: pid_t = pid.unwrap_or(0);
    sd_try!(ffi::sd_pid_get_owner_uid(p, &mut c_owner_uid));
    Ok(c_owner_uid as uid_t)
}
