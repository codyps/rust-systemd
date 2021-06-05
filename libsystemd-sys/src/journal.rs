#![allow(non_camel_case_types)]

use super::size_t;
use super::{c_char, c_int, c_void, ConstIovec};

pub const SD_JOURNAL_LOCAL_ONLY: c_int = 1 << 0;
pub const SD_JOURNAL_RUNTIME_ONLY: c_int = 1 << 1;
pub const SD_JOURNAL_SYSTEM: c_int = 1 << 2;
pub const SD_JOURNAL_CURRENT_USER: c_int = 1 << 3;
pub const SD_JOURNAL_OS_ROOT: c_int = 1 << 4;
pub const SD_JOURNAL_ALL_NAMESPACES: c_int = 1 << 5;
pub const SD_JOURNAL_INCLUDE_DEFAULT_NAMESPACE: c_int = 1 << 6;

// Wakeup event types
pub const SD_JOURNAL_NOP: c_int = 0;
pub const SD_JOURNAL_APPEND: c_int = 1;
pub const SD_JOURNAL_INVALIDATE: c_int = 2;

use crate::id128::sd_id128_t;
pub enum sd_journal {}

extern "C" {
    // sd-journal
    pub fn sd_journal_sendv(iv: *const ConstIovec, n: c_int) -> c_int;
    // There are a bunch of other send methods, but for rust it doesn't make sense to call them
    // (we don't need to do c-style format strings)

    pub fn sd_journal_open(ret: *mut *mut sd_journal, flags: c_int) -> c_int;
    #[cfg(feature = "systemd_v245")]
    pub fn sd_journal_open_namespace(
        ret: *mut *mut sd_journal,
        namespace: *const c_char,
        flags: c_int,
    ) -> c_int;
    pub fn sd_journal_open_directory(
        ret: *mut *mut sd_journal,
        path: *const c_char,
        flags: c_int,
    ) -> c_int;
    pub fn sd_journal_open_files(
        ret: *mut *mut sd_journal,
        path: *const *const c_char,
        flags: c_int,
    ) -> c_int;
    pub fn sd_journal_close(j: *mut sd_journal);

    pub fn sd_journal_previous(j: *mut sd_journal) -> c_int;
    pub fn sd_journal_next(j: *mut sd_journal) -> c_int;

    pub fn sd_journal_previous_skip(j: *mut sd_journal, skip: u64) -> c_int;
    pub fn sd_journal_next_skip(j: *mut sd_journal, skip: u64) -> c_int;

    pub fn sd_journal_get_realtime_usec(j: *mut sd_journal, ret: *mut u64) -> c_int;
    pub fn sd_journal_get_monotonic_usec(
        j: *mut sd_journal,
        ret: *mut u64,
        ret_boot_id: *mut sd_id128_t,
    ) -> c_int;

    pub fn sd_journal_set_data_threshold(j: *mut sd_journal, sz: size_t) -> c_int;
    pub fn sd_journal_get_data_threshold(j: *mut sd_journal, sz: *mut size_t) -> c_int;

    pub fn sd_journal_get_data(
        j: *mut sd_journal,
        field: *const c_char,
        data: *mut *mut u8,
        l: *mut size_t,
    ) -> c_int;
    pub fn sd_journal_enumerate_data(
        j: *mut sd_journal,
        data: *mut *const u8,
        l: *mut size_t,
    ) -> c_int;
    pub fn sd_journal_restart_data(j: *mut sd_journal);

    pub fn sd_journal_add_match(j: *mut sd_journal, data: *const c_void, size: size_t) -> c_int;
    pub fn sd_journal_add_disjunction(j: *mut sd_journal) -> c_int;
    pub fn sd_journal_add_conjunction(j: *mut sd_journal) -> c_int;
    pub fn sd_journal_flush_matches(j: *mut sd_journal);

    pub fn sd_journal_seek_head(j: *mut sd_journal) -> c_int;
    pub fn sd_journal_seek_tail(j: *mut sd_journal) -> c_int;
    pub fn sd_journal_seek_monotonic_usec(
        j: *mut sd_journal,
        boot_id: sd_id128_t,
        usec: u64,
    ) -> c_int;
    pub fn sd_journal_seek_realtime_usec(j: *mut sd_journal, usec: u64) -> c_int;
    pub fn sd_journal_seek_cursor(j: *mut sd_journal, cursor: *const c_char) -> c_int;

    pub fn sd_journal_get_cursor(j: *mut sd_journal, cursor: *mut *const c_char) -> c_int;
    pub fn sd_journal_test_cursor(j: *mut sd_journal, cursor: *const c_char) -> c_int;

    pub fn sd_journal_get_cutoff_realtime_usec(
        j: *mut sd_journal,
        from: *mut u64,
        to: *mut u64,
    ) -> c_int;
    pub fn sd_journal_get_cutoff_monotonic_usec(
        j: *mut sd_journal,
        boot_id: sd_id128_t,
        from: *mut u64,
        to: *mut u64,
    ) -> c_int;

    pub fn sd_journal_get_usage(j: *mut sd_journal, bytes: *mut u64) -> c_int;

    pub fn sd_journal_query_unique(j: *mut sd_journal, field: *const c_char) -> c_int;
    pub fn sd_journal_enumerate_unique(
        j: *mut sd_journal,
        data: *mut *const c_void,
        l: *mut size_t,
    ) -> c_int;
    pub fn sd_journal_restart_unique(j: *mut sd_journal);

    pub fn sd_journal_get_fd(j: *mut sd_journal) -> c_int;
    pub fn sd_journal_get_events(j: *mut sd_journal) -> c_int;
    pub fn sd_journal_get_timeout(j: *mut sd_journal, timeout_usec: *mut u64) -> c_int;
    pub fn sd_journal_process(j: *mut sd_journal) -> c_int;
    pub fn sd_journal_wait(j: *mut sd_journal, timeout_usec: u64) -> c_int;
    pub fn sd_journal_reliable_fd(j: *mut sd_journal) -> c_int;

    pub fn sd_journal_get_catalog(j: *mut sd_journal, text: *mut *const c_char) -> c_int;
    pub fn sd_journal_get_catalog_for_message_id(id: sd_id128_t, ret: *mut *const c_char) -> c_int;
}
