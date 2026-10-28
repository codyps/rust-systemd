/// High-level interface to the systemd daemon module.
///
/// Note that most, if not all, of these APIs can be found in the pure-rust
/// [libsystemd](https://crates.io/crates/libsystemd) crate, and you may prefer to use it instead.
use super::ffi::{c_int, pid_t, size_t};
use super::{Error, Result};
use crate::ffi_result_with_context as ffi_result;
use crate::CStrArgument;
use ::ffi::daemon as ffi;
use libc::{c_char, c_uint};
use libc::{SOCK_DGRAM, SOCK_RAW, SOCK_STREAM};
use std::ffi::{CStr, CString};
use std::io::ErrorKind;
use std::net::TcpListener;
use std::os::unix::io::FromRawFd;
use std::os::unix::io::RawFd as Fd;
use std::ptr;
use std::ptr::null;

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
    unsafe fn new(unset_environment: bool) -> Result<Self> {
        let num_fds = ffi_result(
            unsafe { ffi::sd_listen_fds(unset_environment as c_int) },
            "sd_listen_fds",
        )?;
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

/// Returns a struct that can iterate over the passed file descriptors.
/// Leaves the process environment unchanged.
pub fn listen_fds() -> Result<ListenFds> {
    // SAFETY: passing false prevents libsystemd from modifying the environment.
    unsafe { ListenFds::new(false) }
}

/// Like [`listen_fds()`], but also removes the socket activation environment
/// variables, including when libsystemd reports an error.
///
/// # Safety
///
/// The caller must ensure no other thread or signal handler reads or modifies
/// the process environment, including indirectly through C library functions.
/// Call this during single-threaded startup before spawning any threads.
pub unsafe fn listen_fds_unset_environment() -> Result<ListenFds> {
    unsafe { ListenFds::new(true) }
}

/// Socket-activation descriptors paired with their names, in native order.
///
/// This collection owns the names, preserving their original bytes. Descriptor
/// numbers start at 3, as with [`ListenFds`]. Names may repeat; use
/// [`Self::iter_named`] to select every descriptor with a given name.
///
/// The descriptors remain raw file descriptors: dropping this collection does
/// not close them. Repeated calls with the environment retained refer to the
/// same descriptors and do not create independent ownership of them.
#[derive(Debug)]
pub struct ListenFdsWithNames {
    names: Vec<CString>,
}

impl ListenFdsWithNames {
    unsafe fn new(unset_environment: bool) -> Result<Self> {
        let mut native = NativeFdNames(ptr::null_mut());
        // SAFETY: native is a writable output. The caller permits environment
        // mutation when requested; false leaves the environment unchanged.
        let count = ffi_result(
            unsafe { ffi::sd_listen_fds_with_names(unset_environment as c_int, &mut native.0) },
            "sd_listen_fds_with_names",
        )?;
        let mut names = Vec::with_capacity(count as usize);
        if count > 0 {
            // SAFETY: a positive native result supplies count non-null string
            // pointers in a live array owned by native. Avoid a null slice for
            // the zero-descriptor result, which may leave the output null.
            let pointers = unsafe { std::slice::from_raw_parts(native.0, count as usize) };
            for &name in pointers {
                // SAFETY: each string is NUL-terminated and remains live until
                // the guard is dropped. Copy into Rust-owned CString storage;
                // native allocations must be released using libc::free.
                names.push(unsafe { CStr::from_ptr(name) }.to_owned());
            }
        }
        Ok(Self { names })
    }

    /// Number of passed descriptors, including those with duplicate names.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Whether no descriptors were passed to this process.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// Iterate over raw descriptor numbers and borrowed names in native order.
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (Fd, &CStr)> + DoubleEndedIterator + std::iter::FusedIterator + '_
    {
        self.names
            .iter()
            .enumerate()
            // The native count is bounded by INT_MAX - LISTEN_FDS_START.
            .map(|(index, name)| (LISTEN_FDS_START + index as Fd, name.as_c_str()))
    }

    /// Iterate over all descriptor numbers with this name, preserving order.
    ///
    /// Accepts Rust strings, C strings, their owned forms, and `Cow`. A single
    /// trailing NUL is accepted; interior NUL bytes return `InvalidInput`.
    pub fn iter_named<'a>(
        &'a self,
        name: impl CStrArgument<'a>,
    ) -> Result<impl DoubleEndedIterator<Item = Fd> + std::iter::FusedIterator + 'a> {
        let name = name
            .try_into_cstr()
            .map_err(|error| error.with_argument("name"))?;
        Ok(self
            .iter()
            .filter_map(move |(fd, candidate)| (candidate == name.as_ref()).then_some(fd)))
    }
}

// Own the native string vector until every name has been copied, including
// during unwinding. Neither the vector nor its elements use Rust's allocator.
struct NativeFdNames(*mut *mut c_char);

impl Drop for NativeFdNames {
    fn drop(&mut self) {
        if self.0.is_null() {
            return;
        }
        // SAFETY: sd_listen_fds_with_names transfers a malloc-allocated,
        // null-terminated vector of malloc-allocated strings on success.
        // The output starts null and is left unchanged on errors/no activation.
        unsafe {
            let mut current = self.0;
            while !(*current).is_null() {
                libc::free((*current).cast());
                current = current.add(1);
            }
            libc::free(self.0.cast());
        }
    }
}

/// Read socket-activation descriptors and their names without clearing the
/// process environment.
///
/// Corresponds to [`sd_listen_fds_with_names`], available since systemd 227.
/// Reads `LISTEN_PID`, `LISTEN_FDS`, and `LISTEN_FDNAMES` (and `LISTEN_PIDFDID`
/// on supporting versions). No activation for this process returns an empty
/// collection. Missing names are reported as `unknown`; a name-count mismatch
/// returns `EINVAL`. The native call sets `FD_CLOEXEC` on passed descriptors;
/// some flags may already have changed if it later reports an error.
///
/// The collection owns only the names and does not close descriptors on drop.
///
/// [`sd_listen_fds_with_names`]: https://www.freedesktop.org/software/systemd/man/latest/sd_listen_fds.html
///
/// ```
/// use systemd::daemon;
/// let descriptors = daemon::listen_fds_with_names()?;
/// for (fd, name) in descriptors.iter() {
///     println!("descriptor {fd}: {name:?}");
/// }
/// for fd in descriptors.iter_named("http")? {
///     println!("HTTP descriptor: {fd}");
/// }
/// # Ok::<(), systemd::Error>(())
/// ```
pub fn listen_fds_with_names() -> Result<ListenFdsWithNames> {
    // SAFETY: false prevents libsystemd from modifying the environment.
    unsafe { ListenFdsWithNames::new(false) }
}

/// Like [`listen_fds_with_names`], but clears the socket-activation environment
/// variables supported by the installed libsystemd, including on errors.
///
/// # Safety
///
/// The caller must ensure no other thread or signal handler reads or modifies
/// the process environment, including indirectly through C library functions.
/// Call during single-threaded startup before spawning any threads.
///
/// ```no_run
/// use systemd::daemon;
/// // SAFETY: startup code ensures no concurrent environment access.
/// let descriptors = unsafe { daemon::listen_fds_with_names_unset_environment() }?;
/// # Ok::<(), systemd::Error>(())
/// ```
///
/// Clearing the environment requires an explicit unsafe call:
///
/// ```compile_fail
/// let descriptors = systemd::daemon::listen_fds_with_names_unset_environment();
/// ```
pub unsafe fn listen_fds_with_names_unset_environment() -> Result<ListenFdsWithNames> {
    unsafe { ListenFdsWithNames::new(true) }
}

/// Identifies whether the passed file descriptor is a FIFO.  If a path is
/// supplied, the file descriptor must also match the path.
pub fn is_fifo<'a, S: CStrArgument<'a>>(fd: Fd, path: Option<S>) -> Result<bool> {
    let path = path
        .map(|x| {
            x.try_into_cstr()
                .map_err(|error| error.with_argument("path"))
        })
        .transpose()?;
    let result = ffi_result(
        unsafe { ffi::sd_is_fifo(fd, path.as_ref().map_or(null(), |x| x.as_ptr())) },
        "sd_is_fifo",
    )?;
    Ok(result != 0)
}

/// Identifies whether the passed file descriptor is a special character device.
/// If a path is supplied, the file descriptor must also match the path.
pub fn is_special<'a, S: CStrArgument<'a>>(fd: Fd, path: Option<S>) -> Result<bool> {
    let path = path
        .map(|x| {
            x.try_into_cstr()
                .map_err(|error| error.with_argument("path"))
        })
        .transpose()?;
    let result = ffi_result(
        unsafe { ffi::sd_is_special(fd, path.as_ref().map_or(null(), |x| x.as_ptr())) },
        "sd_is_special",
    )?;
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

    let result = ffi_result(
        unsafe { ffi::sd_is_socket(fd, c_family, c_socktype, c_listening) },
        "sd_is_socket",
    )?;
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

    let result = ffi_result(
        unsafe { ffi::sd_is_socket_inet(fd, c_family, c_socktype, c_listening, c_port) },
        "sd_is_socket_inet",
    )?;
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
pub fn is_socket_unix<'a, S: CStrArgument<'a>>(
    fd: Fd,
    socktype: Option<SocketType>,
    listening: Listening,
    path: Option<S>,
) -> Result<bool> {
    let path_cstr = path
        .map(|p| {
            p.try_into_cstr()
                .map_err(|error| error.with_argument("path"))
        })
        .transpose()?;
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

    let result = ffi_result(
        unsafe { ffi::sd_is_socket_unix(fd, c_socktype, c_listening, c_path, c_length) },
        "sd_is_socket_unix",
    )?;
    Ok(result != 0)
}

/// Identifies whether the passed file descriptor is a POSIX message queue. If a
/// path is supplied, it will also verify the name.
pub fn is_mq<'a, S: CStrArgument<'a>>(fd: Fd, path: Option<S>) -> Result<bool> {
    let path = path
        .map(|x| {
            x.try_into_cstr()
                .map_err(|error| error.with_argument("path"))
        })
        .transpose()?;
    let result = ffi_result(
        unsafe { ffi::sd_is_mq(fd, path.as_ref().map_or(null(), |x| x.as_ptr())) },
        "sd_is_mq",
    )?;
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
/// Leaves the process environment unchanged.
pub fn notify<'a, I, K, V>(state: I) -> Result<bool>
where
    I: Iterator<Item = &'a (K, V)>,
    K: AsRef<str> + 'a,
    V: AsRef<str> + 'a,
{
    pid_notify(0, state)
}

/// Like [`notify()`], but also removes `NOTIFY_SOCKET` from the environment.
///
/// # Safety
///
/// The caller must ensure no other thread or signal handler reads or modifies
/// the process environment, including indirectly through C library functions.
/// Call this during single-threaded startup before spawning any threads.
pub unsafe fn notify_unset_environment<'a, I, K, V>(state: I) -> Result<bool>
where
    I: Iterator<Item = &'a (K, V)>,
    K: AsRef<str> + 'a,
    V: AsRef<str> + 'a,
{
    unsafe { pid_notify_unset_environment(0, state) }
}

/// Similar to `notify()`, but this sends the message on behalf of the supplied
/// PID, if possible.
/// Leaves the process environment unchanged.
pub fn pid_notify<'a, I, K, V>(pid: pid_t, state: I) -> Result<bool>
where
    I: Iterator<Item = &'a (K, V)>,
    K: AsRef<str> + 'a,
    V: AsRef<str> + 'a,
{
    pid_notify_with_fds(pid, state, &[])
}

/// Like [`pid_notify()`], but also removes `NOTIFY_SOCKET` from the environment.
///
/// # Safety
///
/// The caller must ensure no other thread or signal handler reads or modifies
/// the process environment, including indirectly through C library functions.
/// Call this during single-threaded startup before spawning any threads.
pub unsafe fn pid_notify_unset_environment<'a, I, K, V>(pid: pid_t, state: I) -> Result<bool>
where
    I: Iterator<Item = &'a (K, V)>,
    K: AsRef<str> + 'a,
    V: AsRef<str> + 'a,
{
    unsafe { pid_notify_with_fds_unset_environment(pid, state, &[]) }
}

/// Similar to `pid_notify()`, but this also sends file descriptors to the store.
/// Leaves the process environment unchanged.
pub fn pid_notify_with_fds<'a, I, K, V>(pid: pid_t, state: I, fds: &[Fd]) -> Result<bool>
where
    I: Iterator<Item = &'a (K, V)>,
    K: AsRef<str> + 'a,
    V: AsRef<str> + 'a,
{
    // SAFETY: passing false prevents libsystemd from modifying the environment.
    unsafe { pid_notify_with_fds_impl(pid, false, state, fds) }
}

/// Like [`pid_notify_with_fds()`], but also removes `NOTIFY_SOCKET` from the environment.
///
/// # Safety
///
/// The caller must ensure no other thread or signal handler reads or modifies
/// the process environment, including indirectly through C library functions.
/// Call this during single-threaded startup before spawning any threads.
pub unsafe fn pid_notify_with_fds_unset_environment<'a, I, K, V>(
    pid: pid_t,
    state: I,
    fds: &[Fd],
) -> Result<bool>
where
    I: Iterator<Item = &'a (K, V)>,
    K: AsRef<str> + 'a,
    V: AsRef<str> + 'a,
{
    unsafe { pid_notify_with_fds_impl(pid, true, state, fds) }
}

unsafe fn pid_notify_with_fds_impl<'a, I, K, V>(
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
    let result = ffi_result(
        unsafe {
            ffi::sd_pid_notify_with_fds(
                pid,
                unset_environment as c_int,
                c_state.as_ptr(),
                fds.as_ptr(),
                fds.len() as c_uint,
            )
        },
        "sd_pid_notify_with_fds",
    )?;
    Ok(result != 0)
}

/// Returns true if the system was booted with systemd.
pub fn booted() -> Result<bool> {
    let result = ffi_result(unsafe { ffi::sd_booted() }, "sd_booted")?;
    Ok(result != 0)
}

/// Returns a timeout in microseconds before which the watchdog expects a
/// response from the process. If 0, the watchdog is disabled.
/// Leaves the process environment unchanged.
pub fn watchdog_enabled() -> Result<u64> {
    // SAFETY: passing false prevents libsystemd from modifying the environment.
    unsafe { watchdog_enabled_impl(false) }
}

/// Like [`watchdog_enabled()`], but also removes `WATCHDOG_USEC` and `WATCHDOG_PID`.
///
/// # Safety
///
/// The caller must ensure no other thread or signal handler reads or modifies
/// the process environment, including indirectly through C library functions.
/// Call this during single-threaded startup before spawning any threads.
pub unsafe fn watchdog_enabled_unset_environment() -> Result<u64> {
    unsafe { watchdog_enabled_impl(true) }
}

unsafe fn watchdog_enabled_impl(unset_environment: bool) -> Result<u64> {
    let mut timeout: u64 = 0;
    ffi_result(
        unsafe { ffi::sd_watchdog_enabled(unset_environment as c_int, &mut timeout) },
        "sd_watchdog_enabled",
    )?;
    Ok(timeout)
}
