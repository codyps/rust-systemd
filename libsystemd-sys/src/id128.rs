use super::{c_char, c_int};

/// Note: this is marked `Copy` because the libsystemd apis pass it by value without implying an
/// ownership transfer.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct sd_id128_t {
    pub bytes: [u8; 16],
}

pub const SD_ID128_STRING_MAX: usize = 33;

extern "C" {
    // s: &[c_char;33]
    pub fn sd_id128_to_string(id: sd_id128_t, s: *mut c_char) -> *mut c_char;

    // s: &[c_char;33]
    pub fn sd_id128_from_string(s: *const c_char, ret: *mut sd_id128_t) -> c_int;

    pub fn sd_id128_randomize(ret: *mut sd_id128_t) -> c_int;
    pub fn sd_id128_get_machine(ret: *mut sd_id128_t) -> c_int;
    pub fn sd_id128_get_boot(ret: *mut sd_id128_t) -> c_int;
}
