/// High-level interface to the systemd daemon module.
///
/// Note that most, if not all, of these APIs can be found in the pure-rust
/// [libsystemd](https://crates.io/crates/libsystemd) crate, and you may prefer to use it instead.
use super::ffi::{c_int, pid_t, size_t};
use super::{Error, Result};
use crate::ffi_result;
use ::ffi::daemon as ffi;
use cstr_argument::CStrArgument;
use libc::{c_char, c_uint};
use libc::{SOCK_DGRAM, SOCK_RAW, SOCK_STREAM};
use std::io::ErrorKind;
use std::net::TcpListener;
use std::os::unix::io::FromRawFd;
use std::os::unix::io::RawFd as Fd;
use std::ptr::null;
use std::{env, ptr};

// XXX: this is stolen from std::old_io::net::addrinfo until we have a replacement in the standard
// lib.
pub enum SocketType {
    Stream,
    Datagram,
    Raw,
}

/// Options for checking whether a socket is in listening mode
pub enum Listening {
    /// Verify that socket is in listening mode
    IsListening,
    /// Verify that socket is not in listening mode
    IsNotListening,
    /// Don't check whether socket is listening
    NoListeningCheck,
}

/// Number of the first passed file descriptor
const LISTEN_FDS_START: Fd = 3;

/// Tells systemd whether daemon startup is finished
pub const STATE_READY: &str = "READY";
/// Tells systemd the daemon is reloading its configuration
pub const STATE_RELOADING: &str = "RELOADING";
/// Tells systemd the daemon is stopping
pub const STATE_STOPPING: &str = "STOPPING";
/// Single-line status string describing daemon state
pub const STATE_STATUS: &str = "STATUS";
/// Errno-style error code in case of failure
pub const STATE_ERRNO: &str = "ERRNO";
/// D-Bus-style error code in case of failure
pub const STATE_BUSERROR: &str = "BUSERROR";
/// Main PID of the daemon, in case systemd didn't fork it itself
pub const STATE_MAINPID: &str = "MAINPID";
/// Update the watchdog timestamp (set to 1). Daemon should do this regularly,
/// if using this feature.
pub const STATE_WATCHDOG: &str = "WATCHDOG";
/// Reset the watchdog timeout during runtime.
pub const STATE_WATCHDOG_USEC: &str = "WATCHDOG_USEC";
/// Extend the timeout for the current state.
pub const STATE_EXTEND_TIMEOUT_USEC: &str = "EXTEND_TIMEOUT_USEC";
/// Store file descriptors in the service manager.
pub const STATE_FDSTORE: &str = "FDSTORE";
/// Remove file descriptors from the service manager store.
pub const STATE_FDSTOREREMOVE: &str = "FDSTOREREMOVE";
/// Name the group of file descriptors sent to the service manager.
pub const STATE_FDNAME: &str = "FDNAME";

/// Represents the result returned by the socket dameon's sd_listen_fds
#[derive(Debug)]
pub struct ListenFds {
    num_fds: c_int,
}

impl ListenFds {
    // Constructs a new set from the number of file_descriptors
    fn new(unset_environment: bool) -> Result<Self> {
        // in order to use rust's locking of the environment, do the env var unsetting ourselves
        let num_fds = ffi_result(unsafe {ffi::sd_listen_fds(0)})?;
        if unset_environment {
            env::remove_var("LISTEN_FDS");
            env::remove_var("LISTEN_PID");
            env::remove_var("LISTEN_FDNAMES");
        }
        Ok(Self { num_fds })
    }

    /// Returns the total number of file descriptors represented by the range
    pub fn len(&self) -> c_int {
        self.num_fds
    }

    /// Returns if no file descriptors were returned
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterable range over the returned file descriptors
    pub fn iter(&self) -> ListenFdsRange {
        ListenFdsRange {
            current: 0,
            num_fds: self.num_fds,
        }
    }
}

/// Provides an iterable range over the passed file descriptors.
#[derive(Clone, Debug)]
pub struct ListenFdsRange {
    current: c_int,
    num_fds: c_int,
}

impl Iterator for ListenFdsRange {
    type Item = Fd;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.num_fds {
            let fd = LISTEN_FDS_START + self.current;
            self.current += 1;
            Some(fd)
        } else {
            None
        }
    }
}

/// Returns a struct that can iterate over the passed file descriptors.  Removes the
/// `$LISTEN_FDS` and `$LISTEN_PID` file descriptors from the environment if
/// `unset_environment` is `true`
pub fn listen_fds(unset_environment: bool) -> Result<ListenFds> {
    ListenFds::new(unset_environment)
}

/// Identifies whether the passed file descriptor is a FIFO.  If a path is
/// supplied, the file descriptor must also match the path.
pub fn is_fifo<S: CStrArgument>(fd: Fd, path: Option<S>) -> Result<bool> {
    let path = path.map(|x| x.into_cstr());
    let result = ffi_result(unsafe {ffi::sd_is_fifo(
        fd,
        path.map_or(null(), |x| x.as_ref().as_ptr())
    )})?;
    Ok(result != 0)
}

/// Identifies whether the passed file descriptor is a special character device.
/// If a path is supplied, the file descriptor must also match the path.
pub fn is_special<S: CStrArgument>(fd: Fd, path: Option<S>) -> Result<bool> {
    let path = path.map(|x| x.into_cstr());
    let result = ffi_result(unsafe {ffi::sd_is_special(
        fd,
        path.map_or(null(), |x| x.as_ref().as_ptr())
    )})?;
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
        None => 0,
    }
}

#[inline]
/// Converts listening mode to the correct flag
fn get_c_listening(listening: Listening) -> c_int {
    match listening {
        Listening::IsListening => 1,
        Listening::IsNotListening => 0,
        Listening::NoListeningCheck => -1,
    }
}

/// Identifies whether the passed file descriptor is a socket. If family and
/// type are supplied, they must match as well. See `Listening` for listening
/// check parameters.
pub fn is_socket(
    fd: Fd,
    family: Option<c_uint>,
    socktype: Option<SocketType>,
    listening: Listening,
) -> Result<bool> {
    let c_family = family.unwrap_or(0) as c_int;
    let c_socktype = get_c_socktype(socktype);
    let c_listening = get_c_listening(listening);

    let result = ffi_result(unsafe {ffi::sd_is_socket(fd, c_family, c_socktype, c_listening)})?;
    Ok(result != 0)
}

/// Identifies whether the passed file descriptor is an Internet socket. If
/// family, type, and/or port are supplied, they must match as well. See
/// `Listening` for listening check parameters.
pub fn is_socket_inet(
    fd: Fd,
    family: Option<c_uint>,
    socktype: Option<SocketType>,
    listening: Listening,
    port: Option<u16>,
) -> Result<bool> {
    let c_family = family.unwrap_or(0) as c_int;
    let c_socktype = get_c_socktype(socktype);
    let c_listening = get_c_listening(listening);
    let c_port = port.unwrap_or(0);

    let result = ffi_result(unsafe {ffi::sd_is_socket_inet(
        fd,
        c_family,
        c_socktype,
        c_listening,
        c_port
    )})?;
    Ok(result != 0)
}

pub fn tcp_listener(fd: Fd) -> Result<TcpListener> {
    if !is_socket_inet(
        fd,
        None,
        Some(SocketType::Stream),
        Listening::IsListening,
        None,
    )? {
        Err(Error::new(
            ErrorKind::InvalidInput,
            "Socket type was not as expected",
        ))
    } else {
        Ok(unsafe { TcpListener::from_raw_fd(fd) })
    }
}

/// Identifies whether the passed file descriptor is an AF_UNIX socket. If type
/// are supplied, it must match as well. For normal sockets, leave the path set
/// to None; otherwise, pass in the full socket path.  See `Listening` for
/// listening check parameters.
pub fn is_socket_unix<S: CStrArgument>(
    fd: Fd,
    socktype: Option<SocketType>,
    listening: Listening,
    path: Option<S>,
) -> Result<bool> {
    let path_cstr = path.map(|p| p.into_cstr());
    let c_socktype = get_c_socktype(socktype);
    let c_listening = get_c_listening(listening);
    let c_path: *const c_char;
    let c_length: size_t;
    match path_cstr.as_ref() {
        Some(p) => {
            let path_ref = p.as_ref();
            c_length = path_ref.to_bytes().len() as size_t;
            c_path = path_ref.as_ptr() as *const c_char;
        }
        None => {
            c_path = ptr::null();
            c_length = 0;
        }
    }

    let result = ffi_result(unsafe {ffi::sd_is_socket_unix(
        fd,
        c_socktype,
        c_listening,
        c_path,
        c_length
    )})?;
    Ok(result != 0)
}

/// Identifies whether the passed file descriptor is a POSIX message queue. If a
/// path is supplied, it will also verify the name.
pub fn is_mq<S: CStrArgument>(fd: Fd, path: Option<S>) -> Result<bool> {
    let path = path.map(|x| x.into_cstr());
    let result = ffi_result(unsafe {ffi::sd_is_mq(
        fd,
        path.map_or(null(), |x| x.as_ref().as_ptr())
    )})?;
    Ok(result != 0)
}
/// Converts a state map to a C-string for notify
fn state_to_c_string<'a, I, K, V>(state: I) -> ::std::ffi::CString
where
    I: Iterator<Item = &'a (K, V)>,
    K: AsRef<str> + 'a,
    V: AsRef<str> + 'a,
{
    let mut state_vec = Vec::new();
    for (key, value) in state {
        state_vec.push([key.as_ref(), value.as_ref()].join("="));
    }
    let state_str = state_vec.join("\n");
    ::std::ffi::CString::new(state_str.as_bytes()).unwrap()
}

/// Notifies systemd that daemon state has changed.  state is made up of a set
/// of key-value pairs.  See `sd-daemon.h` for details. Some of the most common
/// keys are defined as `STATE_*` constants in this module. Returns `true` if
/// systemd was contacted successfully.
pub fn notify<'a, I, K, V>(unset_environment: bool, state: I) -> Result<bool>
where
    I: Iterator<Item = &'a (K, V)>,
    K: AsRef<str> + 'a,
    V: AsRef<str> + 'a,
{
    let c_state = state_to_c_string(state);
    let result = ffi_result(unsafe {ffi::sd_notify(unset_environment as c_int, c_state.as_ptr())})?;
    Ok(result != 0)
}

/// Similar to `notify()`, but this sends the message on behalf of the supplied
/// PID, if possible.
pub fn pid_notify<'a, I, K, V>(pid: pid_t, unset_environment: bool, state: I) -> Result<bool>
where
    I: Iterator<Item = &'a (K, V)>,
    K: AsRef<str> + 'a,
    V: AsRef<str> + 'a,
{
    let c_state = state_to_c_string(state);
    let result = ffi_result(unsafe {ffi::sd_pid_notify(
        pid,
        unset_environment as c_int,
        c_state.as_ptr()
    )})?;
    Ok(result != 0)
}

/// Similar to `pid_notify()`, but this also sends file descriptors to the store.
pub fn pid_notify_with_fds<'a, I, K, V>(
    pid: pid_t,
    unset_environment: bool,
    state: I,
    fds: &[Fd],
) -> Result<bool>
where
    I: Iterator<Item = &'a (K, V)>,
    K: AsRef<str> + 'a,
    V: AsRef<str> + 'a,
{
    let c_state = state_to_c_string(state);
    let result = ffi_result(unsafe {ffi::sd_pid_notify_with_fds(
        pid,
        unset_environment as c_int,
        c_state.as_ptr(),
        fds.as_ptr(),
        fds.len() as c_uint
    )})?;
    Ok(result != 0)
}

/// Returns true if the system was booted with systemd.
pub fn booted() -> Result<bool> {
    let result = ffi_result(unsafe {ffi::sd_booted()})?;
    Ok(result != 0)
}

/// Returns a timeout in microseconds before which the watchdog expects a
/// response from the process. If 0, the watchdog is disabled.
pub fn watchdog_enabled(unset_environment: bool) -> Result<u64> {
    let mut timeout: u64 = 0;
    ffi_result(unsafe {ffi::sd_watchdog_enabled(
        unset_environment as c_int,
        &mut timeout
    )})?;
    Ok(timeout)
}
