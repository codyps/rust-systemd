use super::{c_char, c_int};

#[repr(C)]
pub struct sd_id128_t {
    bytes: [u8;16]
}

extern "C" {
    /* s: &[c_char;33] (note: probably non-mut */
    pub fn sd_id128_to_string(id: sd_id128_t, s: *mut c_char) -> *mut c_char;

    pub fn sd_id128_from_string(s: *const c_char, ret: *mut sd_id128_t) -> c_int;
    pub fn sd_id128_randomize(ret: *mut sd_id128_t) -> c_int;
    pub fn sd_id128_get_machine(ret: *mut sd_id128_t) -> c_int;
    pub fn sd_id128_get_boot(ret: *mut sd_id128_t) -> c_int;
}
