//! Contains definitions for low-level bindings.
//!
//! Most of this module is Rust versions of the systemd headers. The goal of
//! this crate is to make it unattractive to ever use the FFI directly, but
//! it's there if you need it.
//!
//! Items in this module corresponding to systemd functions are well-documented
//! by the systemd man pages.

#![allow(non_camel_case_types)]

extern crate libc;
pub use libc::{size_t,pid_t,uid_t,gid_t};
pub use std::os::raw::{c_char,c_int,c_void,c_uint};

pub const SD_JOURNAL_LOCAL_ONLY:   c_int = 1;
pub const SD_JOURNAL_RUNTIME_ONLY: c_int = 2;
pub const SD_JOURNAL_SYSTEM:       c_int = 4;
pub const SD_JOURNAL_CURRENT_USER: c_int = 8;

#[repr(C)]
pub struct iovec {
    pub iov_base: *mut c_void,
    pub iov_len: size_t
}

#[repr(C)]
pub struct const_iovec {
    pub iov_base: *const c_void,
    pub iov_len: size_t
}

pub fn array_to_iovecs(args: &[&str]) -> Vec<const_iovec> {
    args.iter().map(|d| {
        const_iovec { iov_base: d.as_ptr() as *const c_void, iov_len: d.len() as size_t }
    }).collect()
}

use id128::sd_id128_t;
pub type sd_journal = *mut c_void;

extern {
    /* sd-journal */
    pub fn sd_journal_sendv(iv : *const const_iovec, n : c_int) -> c_int;
    /* There are a bunch of other send methods, but for rust it doesn't make sense to call them
     * (we don't need to do c-style format strings) */

    pub fn sd_journal_open(ret: *const sd_journal, flags: c_int) -> c_int;
    pub fn sd_journal_close(j: sd_journal) -> ();

    pub fn sd_journal_previous(j: sd_journal) -> c_int;
    pub fn sd_journal_next(j: sd_journal) -> c_int;

    pub fn sd_journal_previous_skip(j: sd_journal, skip: u64) -> c_int;
    pub fn sd_journal_next_skip(j: sd_journal, skip: u64) -> c_int;

    pub fn sd_journal_get_realtime_usec(j: sd_journal, ret: *const u64) -> c_int;
    pub fn sd_journal_get_monotonic_usec(j: sd_journal, ret: *const u64, ret_boot_id: *const sd_id128_t) -> c_int;

    pub fn sd_journal_set_data_threshold(j: sd_journal, sz: size_t) -> c_int;
    pub fn sd_journal_get_data_threshold(j: sd_journal, sz: *mut size_t) -> c_int;

    pub fn sd_journal_get_data(j: sd_journal, field: *const c_char, data: *const *mut u8, l: *mut size_t) -> c_int;
    pub fn sd_journal_enumerate_data(j: sd_journal, data: *const *mut u8, l: *mut size_t) -> c_int;
    pub fn sd_journal_restart_data(j: sd_journal) -> ();

    pub fn sd_journal_add_match(j: sd_journal, data: *const c_void, size: size_t) -> c_int;
    pub fn sd_journal_add_disjunction(j: sd_journal) -> c_int;
    pub fn sd_journal_add_conjunction(j: sd_journal) -> c_int;
    pub fn sd_journal_flush_matches(j: sd_journal) -> ();

    pub fn sd_journal_seek_head(j: sd_journal) -> c_int;
    pub fn sd_journal_seek_tail(j: sd_journal) -> c_int;
    pub fn sd_journal_seek_monotonic_usec(j: sd_journal, boot_id: sd_id128_t, usec: u64) -> c_int;
    pub fn sd_journal_seek_realtime_usec(j: sd_journal, usec: u64) -> c_int;
    pub fn sd_journal_seek_cursor(j: sd_journal, cursor: *const c_char) -> c_int;

    pub fn sd_journal_get_cursor(j: sd_journal, cursor: *const *mut c_char) -> c_int;
    pub fn sd_journal_test_cursor(j: sd_journal, cursor: *const c_char) -> c_int;

    pub fn sd_journal_get_cutoff_realtime_usec(j: sd_journal, from: *mut u64, to: *mut u64) -> c_int;
    pub fn sd_journal_get_cutoff_monotonic_usec(j: sd_journal, boot_id: sd_id128_t, from: *mut u64, to: *mut u64) -> c_int;

    pub fn sd_journal_get_usage(j: sd_journal, bytes: *mut u64) -> c_int;

    pub fn sd_journal_query_unique(j: sd_journal, field: *const c_char) -> c_int;
    pub fn sd_journal_enumerate_unique(j: sd_journal, data: *const *mut c_void, l: *mut size_t) -> c_int;
    pub fn sd_journal_restart_unique(j: sd_journal) -> ();

    pub fn sd_journal_get_fd(j: sd_journal) -> c_int;
    pub fn sd_journal_get_events(j: sd_journal) -> c_int;
    pub fn sd_journal_get_timeout(j: sd_journal, timeout_usec: *mut u64) -> c_int;
    pub fn sd_journal_process(j: sd_journal) -> c_int;
    pub fn sd_journal_wait(j: sd_journal, timeout_usec: u64) -> c_int;
    pub fn sd_journal_reliable_fd(j: sd_journal) -> c_int;

    pub fn sd_journal_get_catalog(j: sd_journal, text: *const *mut c_char) -> c_int;
    pub fn sd_journal_get_catalog_for_message_id(id: sd_id128_t, ret: *const *mut c_char) -> c_int;

    /* sd-daemon */
    pub fn sd_listen_fds(unset_environment: c_int) -> c_int;
    pub fn sd_is_fifo(fd: c_int, path: *const c_char) -> c_int;
    pub fn sd_is_special(fd: c_int, path: *const c_char) -> c_int;
    pub fn sd_is_socket(fd: c_int, family: c_int, sock_type: c_int, listening: c_int) -> c_int;
    pub fn sd_is_socket_inet(fd: c_int, family: c_int, sock_type: c_int, listening: c_int, port: u16) -> c_int;
    pub fn sd_is_socket_unix(fd: c_int, sock_type: c_int, listening: c_int, path: *const c_char, length: size_t) -> c_int;
    pub fn sd_is_mq(fd: c_int, path: *const c_char) -> c_int;
    pub fn sd_notify(unset_environment: c_int, state: *const c_char) -> c_int;
    // skipping sd_*notifyf; ignoring format strings
    pub fn sd_pid_notify(pid: pid_t, unset_environment: c_int, state: *const c_char) -> c_int;
    pub fn sd_booted() -> c_int;
    pub fn sd_watchdog_enabled(unset_environment: c_int, usec: *mut u64) -> c_int;



}

pub mod id128;
pub mod event;

#[cfg(feature = "bus")]
pub mod bus;


