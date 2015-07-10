use libc::{c_int,c_uint,size_t};
use std::{ptr,collections};
use std::os::unix::io::RawFd as Fd;
use libc::consts::os::bsd44::{SOCK_STREAM, SOCK_DGRAM, SOCK_RAW};
use libc::types::os::arch::posix88::pid_t;
use std::net::TcpListener;
use ffi;
use super::{Result, Error};
use std::io::ErrorKind;
use std::os::unix::io::FromRawFd;

// XXX: this is stolen from std::old_io::net::addrinfo until we have a replacement in the standard
// lib.
pub enum SocketType {
    Stream,
    Datagram,
    Raw
}

/// Options for checking whether a socket is in listening mode
pub enum Listening {
    /// Verify that socket is in listening mode
    IsListening,
    /// Verify that socket is not in listening mode
    IsNotListening,
    /// Don't check whether socket is listening
    NoListeningCheck
}

/// Number of the first passed file descriptor
pub const LISTEN_FDS_START: Fd = 3;

/// Tells systemd whether daemon startup is finished
pub const STATE_READY: &'static str = "READY";
/// Single-line status string describing daemon state
pub const STATE_STATUS: &'static str = "STATUS";
/// Errno-style error code in case of failure
pub const STATE_ERRNO: &'static str = "ERRNO";
/// D-Bus-style error code in case of failure
pub const STATE_BUSERROR: &'static str = "BUSERROR";
/// Main PID of the daemon, in case systemd didn't fork it itself
pub const STATE_MAINPID: &'static str = "MAINPID";
/// Update the watchdog timestamp (set to 1). Daemon should do this regularly,
/// if using this feature.
pub const STATE_WATCHDOG: &'static str = "WATCHDOG";

/// Returns how many file descriptors have been passed. Removes the
/// `$LISTEN_FDS` and `$LISTEN_PID` file descriptors from the environment if
/// `unset_environment` is `true`
pub fn listen_fds(unset_environment: bool) -> Result<Fd> {
    let fds = sd_try!(ffi::sd_listen_fds(unset_environment as c_int));
    Ok(fds)
}

/// Identifies whether the passed file descriptor is a FIFO.  If a path is
/// supplied, the file descriptor must also match the path.
pub fn is_fifo(fd: Fd, path: Option<&str>) -> Result<bool> {
    let c_path = char_or_null!(path);
    let result = sd_try!(ffi::sd_is_fifo(fd, c_path));
    Ok(result != 0)
}

/// Identifies whether the passed file descriptor is a special character device.
/// If a path is supplied, the file descriptor must also match the path.
pub fn is_special(fd: Fd, path: Option<&str>) -> Result<bool> {
    let c_path = char_or_null!(path);
    let result = sd_try!(ffi::sd_is_special(fd, c_path));
    Ok(result != 0)
}

#[inline]
/// Converts an optional socket type to the correct constant, or 0 for no type
/// check
fn get_c_socktype(socktype: Option<SocketType>) -> c_int {
    match socktype {
        Some(SocketType::Stream) => SOCK_STREAM,
        Some(SocketType::Datagram) => SOCK_DGRAM,
        Some(SocketType::Raw) => SOCK_RAW,
        None => 0
    }
}

#[inline]
/// Converts listening mode to the correct flag
fn get_c_listening(listening: Listening) -> c_int {
    match listening {
        Listening::IsListening => 1,
        Listening::IsNotListening => 0,
        Listening::NoListeningCheck => -1
    }
}

/// Identifies whether the passed file descriptor is a socket. If family and
/// type are supplied, they must match as well. See `Listening` for listening
/// check parameters.
pub fn is_socket(fd: Fd, family: Option<c_uint>, socktype: Option<SocketType>, listening: Listening) -> Result<bool> {
    let c_family = family.unwrap_or(0) as c_int;
    let c_socktype = get_c_socktype(socktype);
    let c_listening = get_c_listening(listening);

    let result = sd_try!(ffi::sd_is_socket(fd, c_family, c_socktype, c_listening));
    Ok(result != 0)
}

/// Identifies whether the passed file descriptor is an Internet socket. If
/// family, type, and/or port are supplied, they must match as well. See
/// `Listening` for listening check parameters.
pub fn is_socket_inet(fd: Fd, family: Option<c_uint>, socktype: Option<SocketType>, listening: Listening, port: Option<u16>) -> Result<bool> {
    let c_family = family.unwrap_or(0) as c_int;
    let c_socktype = get_c_socktype(socktype);
    let c_listening = get_c_listening(listening);
    let c_port = port.unwrap_or(0) as u16;

    let result = sd_try!(ffi::sd_is_socket_inet(fd, c_family, c_socktype, c_listening, c_port));
    Ok(result != 0)
}

pub fn tcp_listener(fd: Fd) -> Result<TcpListener> {
    if ! try!(is_socket_inet(fd, None, Some(SocketType::Stream), Listening::IsListening, None)) {
        Err(Error::new(ErrorKind::InvalidInput, "Socket type was not as expected"))
    } else {
        Ok(unsafe { TcpListener::from_raw_fd(fd) })
    }
}

/// Identifies whether the passed file descriptor is an AF_UNIX socket. If type
/// are supplied, it must match as well. For normal sockets, leave the path set
/// to None; otherwise, pass in the full socket path.  See `Listening` for
/// listening check parameters.
pub fn is_socket_unix(fd: Fd, socktype: Option<SocketType>, listening: Listening, path: Option<&str>) -> Result<bool> {
    let c_socktype = get_c_socktype(socktype);
    let c_listening = get_c_listening(listening);
    let c_path: *const i8;
    let c_length: size_t;
    match path {
        Some(p) => {
            let path_cstr = ::std::ffi::CString::new(p.as_bytes()).unwrap();
            c_length = path_cstr.as_bytes().len() as size_t;
            c_path = path_cstr.as_ptr();
        },
        None => {
            c_path = ptr::null();
            c_length = 0;
        }
    }

    let result = sd_try!(ffi::sd_is_socket_unix(fd, c_socktype, c_listening, c_path, c_length));
    Ok(result != 0)
}

/// Identifies whether the passed file descriptor is a POSIX message queue. If a
/// path is supplied, it will also verify the name.
pub fn is_mq(fd: Fd, path: Option<&str>) -> Result<bool> {
    let c_path = char_or_null!(path);
    let result = sd_try!(ffi::sd_is_mq(fd, c_path));
    Ok(result != 0)
}
/// Converts a state map to a C-string for notify
fn state_to_c_string(state: collections::HashMap<&str, &str>) -> ::std::ffi::CString {
    let mut state_vec = Vec::new();
    for (key, value) in state.iter() {
        state_vec.push(vec![*key, *value].connect("="));
    }
    let state_str = state_vec.connect("\n");
    ::std::ffi::CString::new(state_str.as_bytes()).unwrap()
}

/// Notifies systemd that daemon state has changed.  state is made up of a set
/// of key-value pairs.  See `sd-daemon.h` for details. Some of the most common
/// keys are defined as `STATE_*` constants in this module. Returns `true` if
/// systemd was contacted successfully.
pub fn notify(unset_environment: bool, state: collections::HashMap<&str, &str>) -> Result<bool> {
    let c_state = state_to_c_string(state).as_ptr();
    let result = sd_try!(ffi::sd_notify(unset_environment as c_int, c_state));
    Ok(result != 0)
}

/// Similar to `notify()`, but this sends the message on behalf of the supplied
/// PID, if possible.
pub fn pid_notify(pid: pid_t, unset_environment: bool, state: collections::HashMap<&str, &str>) -> Result<bool> {
    let c_state = state_to_c_string(state).as_ptr();
    let result = sd_try!(ffi::sd_pid_notify(pid, unset_environment as c_int, c_state));
    Ok(result != 0)
}

/// Returns true if the system was booted with systemd.
pub fn booted() -> Result<bool> {
    let result = sd_try!(ffi::sd_booted());
    Ok(result != 0)
}

/// Returns a timeout in microseconds before which the watchdog expects a
/// response from the process. If 0, the watchdog is disabled.
pub fn watchdog_enabled(unset_environment: bool) -> Result<u64> {
    let mut timeout: u64 = 0;
    sd_try!(ffi::sd_watchdog_enabled(unset_environment as c_int, &mut timeout));
    Ok(timeout)
}
