use super::super::{c_char, size_t};
use super::{sd_bus_message_handler_t, sd_bus_property_get_t, sd_bus_property_set_t};
use cfg_if::cfg_if;

// XXX: check this repr, might vary based on platform type sizes
#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct sd_bus_vtable {
    type_and_flags: u64,
    // NOTE: assumes that usize == pointer size == size_t
    union_data: [usize; 5],
}

// FIXME: all this nasty `target_endian` stuff is because we don't have an abstraction for
// bitfields. `c2rust-bitfields` only supports little endian. None of the other bitfield crates
// claim compatibility with C (which is what we require here).
impl sd_bus_vtable {
    pub fn type_and_flags(typ: u8, flags: u64) -> u64 {
        assert!(flags <= ((1 << 56) - 1));

        cfg_if! {
            if #[cfg(target_endian = "little")] {
                (flags << 8) | typ as u64
            } else if #[cfg(target_endian = "big")] {
                ((typ as u64) << 56) | flags
            } else {
                compile_error!("unsupported target_endian")
            }
        }
    }

    pub fn typ(&self) -> u8 {
        cfg_if! {
            if #[cfg(target_endian = "little")] {
                (self.type_and_flags & 0xFF) as u8
            } else if #[cfg(target_endian = "big")] {
                (self.type_and_flags >> 56) as u8
            } else {
                compile_error!("unsupported target_endian")
            }
        }
    }

    pub fn flags(&self) -> u64 {
        cfg_if! {
            if #[cfg(target_endian = "little")] {
                self.type_and_flags >> 8
            } else if #[cfg(target_endian = "big")] {
                self.type_and_flags & 0xFFFFFFFFFFFFFF
            } else {
                compile_error!("unsupported target_endian")
            }
        }
    }
}

#[test]
fn vtable_bitfield() {
    for typ in [0, b'<', b'M', 0xAA, u8::MAX] {
        for flags in [0, 1, 0xBBCCBB, 0xFFFF << 40, (1_u64 << 56) - 1] {
            let b = sd_bus_vtable {
                type_and_flags: sd_bus_vtable::type_and_flags(typ, flags),
                ..Default::default()
            };

            assert_eq!(b.typ(), typ);
            assert_eq!(b.flags(), flags);
            // The first C bitfield occupies the first byte in memory on both
            // little- and big-endian targets.
            assert_eq!(b.type_and_flags.to_ne_bytes()[0], typ);
        }
    }
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
