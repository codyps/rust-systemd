use super::{c_char,c_int};

pub type sd_bus = c_void;
pub type sd_bus_message = c_void;
pub type sd_bus_slot = c_void;
pub type sd_bus_creds = c_void;
pub type sd_bus_track = c_void;

#[repr(C)]
pub struct sd_bus_error {
    pub name: *const c_char,
    pub message: *const c_message,
    pub need_free: c_int,
}

#[repr(C)]
pub struct sd_bus_error_map {
    pub name: *const c_char,
    pub code: c_int,
}

extern "C" {
    /* sd-bus */
    pub fn sd_bus_default(ret: *mut *mut sd_bus) -> c_int;
    pub fn sd_bus_default_user(ret: *mut *mut sd_bus) -> c_int;
    pub fn sd_bus_default_system(ret: *mut *mut sd_bus) -> c_int;

    pub fn sd_bus_open(ret: *mut *mut sd_bus) -> c_int;
    pub fn sd_bus_open_user(ret: *mut *mut sd_bus) -> c_int;
    pub fn sd_bus_open_system(ret: *mut *mut sd_bus) -> c_int;
    pub fn sd_bus_open_system_remote(ret: *mut *mut sd_bus, host: *const c_char) -> c_int;
    pub fn sd_bus_open_system_machine(ret: *mut *mut sd_bus, host: *const c_char) -> c_int;

    pub fn sd_bus_new(ret: *mut *mut sd_bus) -> c_int;

    pub fn sd_bus_set_address(bus: *mut sd_bus) -> c_int;
    pub fn sd_bus_set_fd(bus: *mut sd_bus) -> c_int;
    pub fn sd_bus_set_exec(bus: *mut sd_bus, path: *const c_char, argv: *mut *const c_char) -> c_int;
}
