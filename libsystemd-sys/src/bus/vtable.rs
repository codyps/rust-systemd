use std::mem::{transmute, zeroed};
use std::default::Default;
use super::super::{c_char, size_t};
use super::{sd_bus_message_handler_t, sd_bus_property_get_t, sd_bus_property_set_t};
use c2rust_bitfields::BitfieldStruct;

// XXX: check this repr, might vary based on platform type sizes
#[derive(Clone,Copy,Debug)]
#[repr(u32)]
pub enum SdBusVtableType {
    Start = '<' as u32,
    End = '>' as u32,
    Method = 'M' as u32,
    Signal = 'S' as u32,
    Property = 'P' as u32,
    WritableProperty = 'W' as u32,
}

#[derive(Clone, Copy, Debug)]
#[repr(u64)]
pub enum SdBusVtableFlag {
    #[allow(clippy::identity_op)]
    Deprecated = 1 << 0,
    Hidden = 1 << 1,
    Unprivileged = 1 << 2,
    MethodNoReply = 1 << 3,
    PropertyConst = 1 << 4,
    PropertyEmitsChange = 1 << 5,
    PropertyEmitsInvalidation = 1 << 6,
    PropertyExplicit = 1 << 7,
    CapabilityMask = 0xFFFF << 40,
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct sd_bus_vtable {
    type_and_flags: u64,
    // NOTE: assumes that usize == pointer size == size_t
    union_data: [usize; 5],
}

impl Default for sd_bus_vtable {
    fn default() -> Self {
        unsafe { zeroed() }
    }
}

impl sd_bus_vtable {
    pub fn type_and_flags(typ: u32, flags: u64) -> u64 {
        let mut val = [0u8; 8];
        assert!(typ <= ((1 << 8) - 1));
        assert!(flags <= ((1 << 56) - 1));

        val[0] = typ as u8;
        let flags_raw: [u8; 8] = unsafe { transmute(flags) };
        for i in 0..7 {
            val[i + 1] = flags_raw[i];
        }

        unsafe { transmute(val) }
    }

    // type & flags are stored in a bit field, the ordering of which might change depending on the
    // platform.
    //
    pub fn typ(&self) -> u32 {
        unsafe {
            let raw: *const u8 = &self.type_and_flags as *const _ as *const u8;
            *raw as u32
        }
    }

    pub fn flags(&self) -> u64 {
        // treat the first byte as 0 and the next 7 as their actual values
        let mut val = [0u8; 8];
        unsafe {
            let raw: *const u8 = transmute(&self.type_and_flags);
            for i in 1..8 {
                val[i - 1] = *raw.offset(i as isize);
            }
            transmute(val)
        }
    }
}

#[test]
fn vtable_bitfield() {
    let mut b: sd_bus_vtable = Default::default();

    b.type_and_flags = sd_bus_vtable::type_and_flags(0xAA, 0xBBCCBB);

    assert_eq!(b.typ(), 0xAA);
    assert_eq!(b.flags(), 0xBBCCBB);
}

#[test]
fn size_eq() {
    use std::mem::size_of;
    assert_eq!(size_of::<usize>(), size_of::<size_t>());
    assert_eq!(size_of::<usize>(), size_of::<*const u8>());
}

#[repr(C)]
pub struct sd_bus_table_start {
    pub element_size: size_t,
}

#[repr(C)]
pub struct sd_bus_table_method {
    pub member: *const c_char,
    pub signature: *const c_char,
    pub result: *const c_char,
    pub handler: sd_bus_message_handler_t,
    pub offset: size_t,
}

#[repr(C)]
pub struct sd_bus_table_signal {
    pub member: *const c_char,
    pub signature: *const c_char,
}

#[repr(C)]
pub struct sd_bus_table_property {
    pub member: *const c_char,
    pub signature: *const c_char,
    pub get: sd_bus_property_get_t,
    pub set: sd_bus_property_set_t,
    pub offset: size_t,
}
