//! Typed D-Bus containers: vectors are arrays, tuples are structs, and maps are
//! dictionaries. [`Variant<T>`] wraps one value with its wire signature.
//! Struct tuples have one to twelve fields. Empty arrays and dictionaries keep
//! their element types. Map decoding rejects duplicate keys; use a vector of
//! [`DictEntry`] values when duplicates or wire order need to be preserved.
//!
//! After an encoding error, discard the partially written message. After a
//! decoding error, use [`MessageIter::rewind`] before retrying from the beginning.
//! Typed variants require the expected payload type, including for nested variants.
//!
//! ```
//! use std::collections::BTreeMap;
//! use systemd::bus::{MessageIter, MessageRef};
//! use systemd::bus::types::Variant;
//!
//! fn write(message: &mut MessageRef) -> systemd::Result<()> {
//!     message.append(vec![1u32, 2, 3])?;
//!     message.append(("a struct", Variant(vec![true, false])))?;
//!     let dictionary: BTreeMap<String, Variant<u32>> =
//!         [("count".to_owned(), Variant(3))].iter().cloned().collect();
//!     message.append(dictionary)
//! }
//!
//! fn read(iter: &mut MessageIter<'_>) -> systemd::Result<()> {
//!     let numbers: Vec<u32> = iter.next()?.expect("array");
//!     let structure: (String, Variant<Vec<bool>>) = iter.next()?.expect("struct");
//!     let dictionary: BTreeMap<String, Variant<u32>> = iter.next()?.expect("dictionary");
//!     Ok(())
//! }
//! ```
//!
//! Containers may hold borrowed message data:
//!
//! ```
//! use systemd::bus::MessageRef;
//!
//! fn read_strings(message: &mut MessageRef) -> systemd::Result<Vec<&str>> {
//!     Ok(message.iter()?.next()?.expect("string array"))
//! }
//! ```
//!
//! Those borrows cannot outlive the message that owns them:
//!
//! ```compile_fail
//! use systemd::bus::Message;
//!
//! fn read_strings(mut message: Message) -> systemd::Result<Vec<&'static str>> {
//!     Ok(message.iter()?.next()?.expect("string array"))
//! }
//! ```

use super::{MessageIter, MessageRef};
use crate::bus;
use ffi::{c_char, c_int};
use std::ffi::CStr;
use utf8_cstr::Utf8CStr;

mod containers;
pub use self::containers::{DictEntry, SdBusBasicType, SdBusType, Variant};

/**
 * When impled for a Type, promises that a reference to the type cast to a pointer can be encoded
 * using the given `dbus_type()` using `sd_bus_message_append_basic` and decoded similarly using
 * `sd_bus_message_read_basic`
 *
 * It is unlikely that this is something you want to implement, all the basic types are already
 * implemented internally.
 *
 * NOTE: Ideally, we'd use an associated const here. When those land on stable this may change to
 * that.
 *
 * # Safety
 *
 * If implimented inaccurately, the `ToSdBusMessage` and `FromSdBusMessage` wrapper impls may read
 * or write unexpected regions of memory (as they call sd_bus functions which expect the size of
 * the memory region referred to by the type).
 *
 * Ensure the types implimenting `SdBusMessageDirect` can be safely converted to a pointer and
 * passed to `sd_bus_message_read_basic` and `sd_bus_message_append_basic`.
 */
pub unsafe trait SdBusMessageDirect {
    fn dbus_type() -> u8;
}

/**
 * Allows types to provide a conversion to a dbus message
 */
pub trait ToSdBusMessage {
    /// Append one value. Types used inside typed containers must also implement
    /// [`SdBusType`] with the matching signature.
    fn to_message(&self, m: &mut MessageRef) -> crate::Result<()>;
}

/**
 * Allows types to provide a conversion from a dbus message
 *
 * NOTE: the restriction of `Self: Sized` may cause us to have less than ideal impls sometimes. We
 * may need to add a `from_message_to()` that takes a reference, much like `Clone`.
 */
pub trait FromSdBusMessage<'a> {
    fn from_message(m: &mut MessageIter<'a>) -> crate::Result<Option<Self>>
    where
        Self: Sized;
}

impl<T: SdBusMessageDirect> ToSdBusMessage for T {
    fn to_message(&self, m: &mut MessageRef) -> crate::Result<()> {
        unsafe { m.append_basic_raw(Self::dbus_type(), self as *const _ as *const _) }
    }
}

impl<'a, T: SdBusMessageDirect + 'a> FromSdBusMessage<'a> for T {
    fn from_message(m: &mut MessageIter<'a>) -> crate::Result<Option<Self>>
    where
        Self: Sized,
    {
        let t = Self::dbus_type();
        unsafe { m.read_basic_raw(t, |x| x) }
    }
}

// macro_rules! msg_basic {
//     ($typ:ty $dbus_type:expr) => {
//         impl ToSdBusMessage for $typ {
//             fn to_message(&self, m: &mut MessageRef) -> ::systemdsuper::Result<()> {
//                 let c_type : [u8;2] = [ $dbus_type, '\0' ];
//                 ffi_result(unsafe { ffi::sd_bus_message_append(m, &c_type, self as *const _)})?;
//                 Ok(())
//             }
//         }
//
//         impl FromSdBusMessage for $typ {
//             fn from_message(m: &mut MessageRef) -> ::systemd::Result<Self> {
//                 let c_type : [u8;2] = [ $dbus_type, '\0' ];
//                 let v : Self = unsafe { uninitialized() };
//                 ffi_result(unsafe { ffi::sd_bus_message_read(m, &c_type, &v)})?;
//                 Ok(v)
//             }
//         }
//     }
//
//     ($typ:ty $dbus_type:expr , $($rest:tt)* ) => {
//         msg_basic!($typ $dbus_type);
//         msg_basic!($($rest)*);
//     }
// }

macro_rules! msg_basic {
    ($typ:ty : $dbus_type:expr) => {
        unsafe impl SdBusMessageDirect for $typ {
            fn dbus_type() -> u8 { $dbus_type }
        }
    };

    ($typ:ty : $dbus_type:expr , $($rest:tt)* ) => {
        msg_basic!{$typ : $dbus_type}
        msg_basic!{$($rest)*}
    }
}

msg_basic! {
    u8: b'y',
    i16: b'n',
    u16: b'q',
    i32: b'i',
    u32: b'u',
    i64: b'x',
    u64: b't',
    f64: b'd'
}

impl ToSdBusMessage for bool {
    fn to_message(&self, m: &mut MessageRef) -> crate::Result<()> {
        let i: c_int = if *self { 1 } else { 0 };
        unsafe { m.append_basic_raw(b'b', &i as *const _ as *const _) }?;
        Ok(())
    }
}

impl<'a> FromSdBusMessage<'a> for bool {
    fn from_message(m: &mut MessageIter<'a>) -> crate::Result<Option<Self>>
    where
        Self: Sized,
    {
        unsafe { m.read_basic_raw(b'b', |x: c_int| x != 0) }
    }
}

/**
 * A basic wrapper that simply ensures we send a Fd via the dbus file descriptor mechanisms rather
 * than as a integer
 */
pub struct UnixFd(pub c_int);

impl ToSdBusMessage for UnixFd {
    fn to_message(&self, m: &mut MessageRef) -> crate::Result<()> {
        let i: c_int = self.0;
        unsafe { m.append_basic_raw(b'h', &i as *const _ as *const _) }?;
        Ok(())
    }
}

impl<'a> FromSdBusMessage<'a> for UnixFd {
    fn from_message(m: &mut MessageIter<'a>) -> crate::Result<Option<Self>>
    where
        Self: Sized,
    {
        unsafe { m.read_basic_raw(b'h', UnixFd) }
    }
}

impl ToSdBusMessage for &bus::ObjectPath {
    fn to_message(&self, m: &mut MessageRef) -> crate::Result<()> {
        unsafe { m.append_basic_raw(b'o', self.as_ptr() as *const _) }?;
        Ok(())
    }
}

// String-like values returned by sd_bus_message_read_basic point into the
// sealed message body and may be borrowed for the message's lifetime.
impl<'a> FromSdBusMessage<'a> for &'a bus::ObjectPath {
    fn from_message(m: &mut MessageIter<'a>) -> crate::Result<Option<Self>>
    where
        Self: Sized,
    {
        unsafe {
            m.read_basic_raw(b'o', |x: *const c_char| {
                bus::ObjectPath::from_ptr_unchecked(x)
            })
        }
    }
}

impl ToSdBusMessage for &Utf8CStr {
    fn to_message(&self, m: &mut MessageRef) -> crate::Result<()> {
        unsafe { m.append_basic_raw(b's', self.as_ptr() as *const _) }
    }
}

impl<'a> FromSdBusMessage<'a> for &'a Utf8CStr {
    fn from_message(m: &mut MessageIter<'a>) -> crate::Result<Option<Self>>
    where
        Self: Sized,
    {
        unsafe {
            m.read_basic_raw(b's', |x: *const c_char| {
                Utf8CStr::from_cstr_unchecked(CStr::from_ptr(x))
            })
        }
    }
}
