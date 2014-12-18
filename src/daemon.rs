use libc::{c_int,size_t};
use std::{io,ptr};
use std::os::unix::Fd;
use std::io::net::addrinfo::SocketType;
use libc::consts::os::bsd44::{SOCK_STREAM, SOCK_DGRAM, SOCK_RAW};
use std::num::SignedInt;
use ffi;

/// Options for checking whether a socket is in listening mode
#[deriving(Copy)]
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

/// Returns how many file descriptors have been passed. Removes the
/// `$LISTEN_FDS` and `$LISTEN_PID` file descriptors from the environment if
/// `unset_environment` is `true`
pub fn listen_fds(unset_environment: bool) -> io::IoResult<uint> {
    let fds = sd_try!(ffi::sd_listen_fds(unset_environment as c_int));
    Ok(fds as uint)
}

/// Identifies whether the passed file descriptor is a FIFO.  If a path is
/// supplied, the file descriptor must also match the path.
pub fn is_fifo(fd: Fd, path: Option<&str>) -> io::IoResult<bool> {
    let c_path = char_or_null!(path);
    let result = sd_try!(ffi::sd_is_fifo(fd, c_path));
    Ok(result != 0)
}

/// Identifies whether the passed file descriptor is a special character device.
/// If a path is supplied, the file descriptor must also match the path.
pub fn is_special(fd: Fd, path: Option<&str>) -> io::IoResult<bool> {
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
pub fn is_socket(fd: Fd, family: Option<uint>, socktype: Option<SocketType>, listening: Listening) -> io::IoResult<bool> {
    let c_family = family.unwrap_or(0) as c_int;
    let c_socktype = get_c_socktype(socktype);
    let c_listening = get_c_listening(listening);

    let result = sd_try!(ffi::sd_is_socket(fd, c_family, c_socktype, c_listening));
    Ok(result != 0)
}

/// Identifies whether the passed file descriptor is an Internet socket. If
/// family and type are supplied, they must match as well. See `Listening` for
/// listening check parameters.

pub fn is_socket_inet(fd: Fd, family: Option<uint>, socktype: Option<SocketType>, listening: Listening, port: Option<u16>) -> io::IoResult<bool> {
    let c_family = family.unwrap_or(0) as c_int;
    let c_socktype = get_c_socktype(socktype);
    let c_listening = get_c_listening(listening);
    let c_port = port.unwrap_or(0) as u16;

    let result = sd_try!(ffi::sd_is_socket_inet(fd, c_family, c_socktype, c_listening, c_port));
    Ok(result != 0)
}
