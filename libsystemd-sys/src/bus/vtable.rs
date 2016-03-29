use super::super::{c_char, size_t};
use super::{sd_bus_message_handler_t,sd_bus_property_get_t,sd_bus_property_set_t};

/* XXX: check this repr, might vary based on platform type sizes */
#[derive(Clone,Copy)]
#[repr(u32)]
enum SdBusVtableType {
    Start = '<',
    End = '>',
    Method = 'M',
    Signal = 'S',
    Property = 'P',
    WritableProperty = 'W'
}

#[derive(Clone, Copy)]
#[repr(u64)]
enum SdBusVtableFlag {
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
struct sd_bus_table {
    type_and_flags : u64,
    /* NOTE: assumes that usize == pointer size == size_t */
    union_data: [usize;5],
}

#[derive(Clone)]
#[repr(C)]
struct sd_bus_table_start {
    element_size: size_t,
}

#[derive(Clone)]
#[repr(C)]
struct sd_bus_table_method {
    member: *const c_char,
    signature: *const c_char,
    result: *const c_char,
    handler: sd_bus_message_handler_t,
    offset: size_t
}

#[derive(Clone)]
#[repr(C)]
struct sd_bus_table_signal {
    member: *const c_char,
    signature: *const c_char,
}

#[derive(Clone)]
#[repr(C)]
struct sd_bus_table_property {
    member: *const c_char,
    signature: *const c_char,
    get: sd_bus_property_get_t,
    set: sd_bus_property_set_t,
    offset: size_t,
}
