use super::super::{c_char, size_t};
use super::{sd_bus_message_handler_t,sd_bus_property_get_t,sd_bus_property_set_t};

/* XXX: check this repr, might vary based on platform type sizes */
#[derive(Clone,Copy)]
#[repr(u32)]
pub enum SdBusVtableType {
    Start = '<' as u32,
    End = '>' as u32,
    Method = 'M' as u32,
    Signal = 'S' as u32,
    Property = 'P' as u32,
    WritableProperty = 'W' as u32
}

#[derive(Clone, Copy)]
#[repr(u64)]
pub enum SdBusVtableFlag {
    Deprecated = 1 << 0,
    Hidden = 1 << 1,
    Unprivileged = 1 << 2,
    MethodNoReply = 1 << 3,
    PropertyConst = 1 << 4,
    PropertyEmitsChange = 1 << 5,
    PropertyEmitsInvalidation = 1 << 6,
    PropertyExplicit = 1 << 7,
    CapabilityMask = 0xFFFF << 40
}

#[repr(C)]
pub struct sd_bus_vtable {
    type_and_flags : u64,
    /* NOTE: assumes that usize == pointer size == size_t */
    union_data: [usize;5],
}

#[test]
fn size_eq() {
    assert_eq!(std::mem::size_of::<usize>(), std::mem::size_of::<size_t>());
    assert_eq!(std::mem::size_of::<usize>(), std::mem::size_of::<*const u8>());
}

#[derive(Clone)]
#[repr(C)]
pub struct sd_bus_table_start {
    pub element_size: size_t,
}

#[derive(Clone)]
#[repr(C)]
pub struct sd_bus_table_method {
    pub member: *const c_char,
    pub signature: *const c_char,
    pub result: *const c_char,
    pub handler: sd_bus_message_handler_t,
    pub offset: size_t
}

#[derive(Clone)]
#[repr(C)]
pub struct sd_bus_table_signal {
    pub member: *const c_char,
    pub signature: *const c_char,
}

#[derive(Clone)]
#[repr(C)]
pub struct sd_bus_table_property {
    pub member: *const c_char,
    pub signature: *const c_char,
    pub get: sd_bus_property_get_t,
    pub set: sd_bus_property_set_t,
    pub offset: size_t,
}
