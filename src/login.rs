//! Queries for processes, users, and login sessions.
//!
//! PID queries accept a PID, `Some(pid)`, or `None`. Both `None` and PID `0`
//! select the calling process, following libsystemd's convention. Negative PIDs
//! return an `EINVAL` error.
//!
//! When passing an inferred value such as `Default::default()` or `.into()`,
//! provide an explicit type or pass `None` / `Some(pid)` directly.
//!
//! ```
//! use systemd::login;
//!
//! let _ = login::get_cgroup(1234);
//! let _ = login::get_cgroup(Some(1234));
//! let _ = login::get_cgroup(None);
//! let _ = login::process_cgroup(0);
//! ```

use super::ffi::{c_char, c_uint, pid_t, uid_t};
use super::{free_cstring, Error, Result};
use crate::ffi_result;
use crate::CStrArgument;
use ::ffi::login as ffi;
use std::ptr;

/// Systemd slice and unit types
pub enum UnitType {
    /// User slice, service or scope unit
    UserUnit,
    /// System slice, service or scope unit
    SystemUnit,
}

/// Determines the systemd unit (i.e. service or scope unit) identifier of a process.
///
/// Pass a PID directly or as `Some(pid)` to query a specific process. Pass `None`
/// or `0` to query the calling process.
/// This method can be used to retrieve either a system or an user unit identifier.
///
/// Corresponds to `sd_pid_get_unit()` and `sd_pid_get_user_unit()`.
pub fn get_unit(unit_type: UnitType, pid: impl Into<Option<pid_t>>) -> Result<String> {
    let mut c_unit_name: *mut c_char = ptr::null_mut();
    let p: pid_t = pid.into().unwrap_or(0);
    match unit_type {
        UnitType::UserUnit => {
            ffi_result(unsafe { ffi::sd_pid_get_user_unit(p, &mut c_unit_name) })?
        }
        UnitType::SystemUnit => ffi_result(unsafe { ffi::sd_pid_get_unit(p, &mut c_unit_name) })?,
    };
    let unit_name = unsafe { free_cstring(c_unit_name).unwrap() };
    Ok(unit_name)
}

/// Alias for [`get_unit()`].
pub fn process_unit(unit_type: UnitType, pid: impl Into<Option<pid_t>>) -> Result<String> {
    get_unit(unit_type, pid)
}

/// Determines the slice (either in system or user session) of a process.
///
/// Pass a PID directly or as `Some(pid)` to query a specific process. Pass `None`
/// or `0` to query the calling process.
/// This method can be used to retrieve either a system or an user slice identifier.
///
/// Corresponds to `sd_pid_get_slice()` and `sd_pid_get_user_slice()`.
pub fn get_slice(slice_type: UnitType, pid: impl Into<Option<pid_t>>) -> Result<String> {
    let mut c_slice_name: *mut c_char = ptr::null_mut();
    let p: pid_t = pid.into().unwrap_or(0);
    match slice_type {
        UnitType::UserUnit => {
            ffi_result(unsafe { ffi::sd_pid_get_user_slice(p, &mut c_slice_name) })?
        }
        UnitType::SystemUnit => ffi_result(unsafe { ffi::sd_pid_get_slice(p, &mut c_slice_name) })?,
    };
    let slice_id = unsafe { free_cstring(c_slice_name).unwrap() };
    Ok(slice_id)
}

/// Alias for [`get_slice()`].
pub fn process_slice(slice_type: UnitType, pid: impl Into<Option<pid_t>>) -> Result<String> {
    get_slice(slice_type, pid)
}

/// Determines the machine name of a process.
///
/// Pass a PID directly or as `Some(pid)` to query a specific process. Pass `None`
/// or `0` to query the calling process.
/// This method can be used to retrieve the machine name of processes running
/// inside a VM or a container.
///
/// Corresponds to `sd_pid_get_machine_name()`.
pub fn get_machine_name(pid: impl Into<Option<pid_t>>) -> Result<String> {
    let mut c_machine_name: *mut c_char = ptr::null_mut();
    let p: pid_t = pid.into().unwrap_or(0);
    ffi_result(unsafe { ffi::sd_pid_get_machine_name(p, &mut c_machine_name) })?;
    let machine_id = unsafe { free_cstring(c_machine_name).unwrap() };
    Ok(machine_id)
}

/// Alias for [`get_machine_name()`].
pub fn process_machine_name(pid: impl Into<Option<pid_t>>) -> Result<String> {
    get_machine_name(pid)
}

/// Determines the control group path of a process.
///
/// Pass a PID directly or as `Some(pid)` to query a specific process. Pass `None`
/// or `0` to query the calling process.
/// This method can be used to retrieve the control group path of a specific
/// process, relative to the root of the hierarchy. It returns the path without
/// trailing slash, except for processes located in the root control group,
/// where "/" is returned.
///
/// Corresponds to `sd_pid_get_cgroup()`.
pub fn get_cgroup(pid: impl Into<Option<pid_t>>) -> Result<String> {
    let mut c_cgroup: *mut c_char = ptr::null_mut();
    let p: pid_t = pid.into().unwrap_or(0);
    ffi_result(unsafe { ffi::sd_pid_get_cgroup(p, &mut c_cgroup) })?;
    let cg = unsafe { free_cstring(c_cgroup).unwrap() };
    Ok(cg)
}

/// Alias for [`get_cgroup()`].
pub fn process_cgroup(pid: impl Into<Option<pid_t>>) -> Result<String> {
    get_cgroup(pid)
}

/// Determines the session identifier of a process.
///
/// Pass a PID directly or as `Some(pid)` to query a specific process. Pass `None`
/// or `0` to query the calling process.
/// This method can be used to retrieve a session identifier.
///
/// Corresponds to `sd_pid_get_session()`.
pub fn get_session(pid: impl Into<Option<pid_t>>) -> Result<String> {
    let mut c_session: *mut c_char = ptr::null_mut();
    let p: pid_t = pid.into().unwrap_or(0);
    ffi_result(unsafe { ffi::sd_pid_get_session(p, &mut c_session) })?;
    let ss = unsafe { free_cstring(c_session).unwrap() };
    Ok(ss)
}

/// Alias for [`get_session()`].
pub fn process_session(pid: impl Into<Option<pid_t>>) -> Result<String> {
    get_session(pid)
}

/// Returns the ID of the user's primary session, as selected by logind.
///
/// The returned value is a session ID, not an X11 display address. The `uid`
/// argument identifies the requested user; `0` refers to root.
///
/// Returns an `ENODATA` error if the user has no primary session, or `EINVAL`
/// if the UID is invalid.
///
/// Corresponds to `sd_uid_get_display()`.
pub fn user_primary_session(uid: uid_t) -> Result<String> {
    let mut c_session: *mut c_char = ptr::null_mut();
    ffi_result(unsafe { libsystemd_sys::login::sd_uid_get_display(uid, &mut c_session) })?;
    let ss = unsafe { free_cstring(c_session).unwrap() };
    Ok(ss)
}

/// Determines the seat identifier of the seat the session identified
/// by the specified session identifier belongs to.
///
/// Note that not all sessions are attached to a seat, this call will fail for them.
///
/// Corresponds to `sd_session_get_seat()`.
pub fn get_seat<'a, S: CStrArgument<'a>>(session: S) -> Result<String> {
    let session = session.try_into_cstr()?;
    let mut c_seat: *mut c_char = ptr::null_mut();
    ffi_result(unsafe { ffi::sd_session_get_seat(session.as_ref().as_ptr(), &mut c_seat) })?;
    let ss = unsafe { free_cstring(c_seat).unwrap() };
    Ok(ss)
}

/// Alias for [`get_seat()`].
pub fn session_seat<'a, S: CStrArgument<'a>>(session: S) -> Result<String> {
    get_seat(session)
}

/// Determines the VT number of the session identified by the specified session identifier.
///
/// This function will return an error if the seat does not support VTs.
///
/// Corresponds to `sd_session_get_vt()`.
pub fn get_vt<'a, S: CStrArgument<'a>>(session: S) -> Result<u32> {
    let session = session.try_into_cstr()?;
    let mut vt: c_uint = 0;
    ffi_result(unsafe { ffi::sd_session_get_vt(session.as_ref().as_ptr(), &mut vt) })?;
    Ok(vt)
}

/// Alias for [`get_vt()`].
pub fn session_vt<'a, S: CStrArgument<'a>>(session: S) -> Result<u32> {
    get_vt(session)
}

/// Determines the owner uid of a process.
///
/// Pass a PID directly or as `Some(pid)` to query a specific process. Pass `None`
/// or `0` to query the calling process.
/// This method can be used to retrieve an owner uid.
///
/// Corresponds to `sd_pid_get_owner_uid()`.
pub fn get_owner_uid(pid: impl Into<Option<pid_t>>) -> Result<uid_t> {
    let mut c_owner_uid: u32 = 0u32;
    let p: pid_t = pid.into().unwrap_or(0);
    ffi_result(unsafe { ffi::sd_pid_get_owner_uid(p, &mut c_owner_uid) })?;
    Ok(c_owner_uid as uid_t)
}

/// Alias for [`get_owner_uid()`].
pub fn process_owner_uid(pid: impl Into<Option<pid_t>>) -> Result<uid_t> {
    get_owner_uid(pid)
}

/// Retrieves a list of all active sessions.
///
/// Returns a vector of session identifiers for all currently active sessions
/// in the system. This is useful for enumerating all sessions rather than
/// querying individual sessions.
///
/// Corresponds to `sd_get_sessions()`.
pub fn get_sessions() -> Result<Vec<String>> {
    let mut sessions_ptr: *mut *mut c_char = ptr::null_mut();
    let n_sessions = ffi_result(unsafe { ffi::sd_get_sessions(&mut sessions_ptr) })?;

    if n_sessions == 0 || sessions_ptr.is_null() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::with_capacity(n_sessions as usize);

    unsafe {
        for i in 0..n_sessions {
            let session_ptr = *sessions_ptr.offset(i as isize);
            if !session_ptr.is_null() {
                if let Some(session_id) = free_cstring(session_ptr) {
                    sessions.push(session_id);
                }
            }
        }

        // Free the main array
        ::libc::free(sessions_ptr as *mut ::libc::c_void);
    }

    Ok(sessions)
}

/// Retrieves the user ID (UID) of the specified session.
///
/// Corresponds to `sd_session_get_uid()`.
pub fn get_session_uid<'a, S: CStrArgument<'a>>(session: S) -> Result<uid_t> {
    let session = session.try_into_cstr()?;
    let mut uid: uid_t = 0;
    ffi_result(unsafe { ffi::sd_session_get_uid(session.as_ref().as_ptr(), &mut uid) })?;
    Ok(uid)
}

/// Alias for [`get_session_uid()`].
pub fn session_uid<'a, S: CStrArgument<'a>>(session: S) -> Result<uid_t> {
    get_session_uid(session)
}

/// Retrieves the start time of the specified session as microseconds since Unix epoch.
///
/// Corresponds to `sd_session_get_start_time()`.
pub fn get_session_start_time<'a, S: CStrArgument<'a>>(session: S) -> Result<u64> {
    let session = session.try_into_cstr()?;
    let mut start_time_usec: u64 = 0;
    ffi_result(unsafe {
        ffi::sd_session_get_start_time(session.as_ref().as_ptr(), &mut start_time_usec)
    })?;
    Ok(start_time_usec)
}

/// Alias for [`get_session_start_time()`].
pub fn session_start_time<'a, S: CStrArgument<'a>>(session: S) -> Result<u64> {
    get_session_start_time(session)
}

/// Retrieves the TTY device name of the specified session.
///
/// Returns the TTY device name (e.g., "tty1", "pts/0") for the session.
/// Returns None if the session is not associated with a TTY.
///
/// Corresponds to `sd_session_get_tty()`.
pub fn get_session_tty<'a, S: CStrArgument<'a>>(session: S) -> Result<Option<String>> {
    let session = session.try_into_cstr()?;
    let mut tty_ptr: *mut c_char = ptr::null_mut();
    let result = unsafe { ffi::sd_session_get_tty(session.as_ref().as_ptr(), &mut tty_ptr) };

    if result < 0 {
        if result == -libc::ENODATA {
            return Ok(None); // Session has no TTY, this is not an error
        }
        return Err(Error::from_raw_os_error(-result));
    }

    Ok(unsafe { free_cstring(tty_ptr) })
}

/// Alias for [`get_session_tty()`].
pub fn session_tty<'a, S: CStrArgument<'a>>(session: S) -> Result<Option<String>> {
    get_session_tty(session)
}

/// Retrieves the remote host name of the specified session.
///
/// Returns the remote host name for sessions that were established from
/// a remote location (e.g., SSH sessions). Returns None if the session
/// is local or if no remote host information is available.
///
/// Corresponds to `sd_session_get_remote_host()`.
pub fn get_session_remote_host<'a, S: CStrArgument<'a>>(session: S) -> Result<Option<String>> {
    let session = session.try_into_cstr()?;
    let mut remote_host_ptr: *mut c_char = ptr::null_mut();
    let result =
        unsafe { ffi::sd_session_get_remote_host(session.as_ref().as_ptr(), &mut remote_host_ptr) };

    if result < 0 {
        if result == -libc::ENODATA {
            return Ok(None); // No remote host, this is not an error
        }
        return Err(Error::from_raw_os_error(-result));
    }

    Ok(unsafe { free_cstring(remote_host_ptr) })
}

/// Alias for [`get_session_remote_host()`].
pub fn session_remote_host<'a, S: CStrArgument<'a>>(session: S) -> Result<Option<String>> {
    get_session_remote_host(session)
}

/// Retrieves the display name of the specified session.
///
/// Returns the display identifier (e.g., ":0") for graphical sessions.
/// Returns None if the session is not graphical or has no display.
///
/// Corresponds to `sd_session_get_display()`.
pub fn get_session_display<'a, S: CStrArgument<'a>>(session: S) -> Result<Option<String>> {
    let session = session.try_into_cstr()?;
    let mut display_ptr: *mut c_char = ptr::null_mut();
    let result =
        unsafe { ffi::sd_session_get_display(session.as_ref().as_ptr(), &mut display_ptr) };

    if result < 0 {
        if result == -libc::ENODATA {
            return Ok(None); // No display, this is not an error
        }
        return Err(Error::from_raw_os_error(-result));
    }

    Ok(unsafe { free_cstring(display_ptr) })
}

/// Alias for [`get_session_display()`].
pub fn session_display<'a, S: CStrArgument<'a>>(session: S) -> Result<Option<String>> {
    get_session_display(session)
}

/// Retrieves the session type of the specified session.
///
/// Returns the session type (e.g., "tty", "x11", "wayland", "mir") which
/// indicates the type of session.
///
/// Corresponds to `sd_session_get_type()`.
pub fn get_session_type<'a, S: CStrArgument<'a>>(session: S) -> Result<Option<String>> {
    let session = session.try_into_cstr()?;
    let mut type_ptr: *mut c_char = ptr::null_mut();
    let result = unsafe { ffi::sd_session_get_type(session.as_ref().as_ptr(), &mut type_ptr) };

    if result < 0 {
        if result == -libc::ENODATA {
            return Ok(None); // No type information, this is not an error
        }
        return Err(Error::from_raw_os_error(-result));
    }

    Ok(unsafe { free_cstring(type_ptr) })
}

/// Alias for [`get_session_type()`].
pub fn session_type<'a, S: CStrArgument<'a>>(session: S) -> Result<Option<String>> {
    get_session_type(session)
}
