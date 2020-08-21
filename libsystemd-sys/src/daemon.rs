use super::{c_int, size_t, c_char, c_uint, pid_t};

extern "C" {
    pub fn sd_listen_fds(unset_environment: c_int) -> c_int;
    pub fn sd_is_fifo(fd: c_int, path: *const c_char) -> c_int;
    pub fn sd_is_special(fd: c_int, path: *const c_char) -> c_int;
    pub fn sd_is_socket(fd: c_int, family: c_int, sock_type: c_int, listening: c_int) -> c_int;
    pub fn sd_is_socket_inet(fd: c_int,
                             family: c_int,
                             sock_type: c_int,
                             listening: c_int,
                             port: u16)
                             -> c_int;
    pub fn sd_is_socket_unix(fd: c_int,
                             sock_type: c_int,
                             listening: c_int,
                             path: *const c_char,
                             length: size_t)
                             -> c_int;
    // On elogind it always returns error.
    #[cfg(not(feature = "elogind"))]
    pub fn sd_is_mq(fd: c_int, path: *const c_char) -> c_int;
    pub fn sd_notify(unset_environment: c_int, state: *const c_char) -> c_int;
    // skipping sd_*notifyf; ignoring format strings
    pub fn sd_pid_notify(pid: pid_t, unset_environment: c_int, state: *const c_char) -> c_int;
    pub fn sd_pid_notify_with_fds(pid: pid_t, unset_environment: c_int, state: *const c_char, fds: *const c_int, n_fds: c_uint) -> c_int;
    pub fn sd_booted() -> c_int;
    pub fn sd_watchdog_enabled(unset_environment: c_int, usec: *mut u64) -> c_int;
}
