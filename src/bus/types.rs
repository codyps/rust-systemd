/*!
 * Define a mechanism for converting types to messages and messages to types
 *
 * A few existing models:
 *
 * - rust-postgres: defines ToSql and FromSql traits, and has their accessor and creator functions
 *   take slices of trait objects. rust-postgress is not an ffi lib, so they read directly from a
 *   Read and write to a Write. For them, each item is associated with a postgres type, so they
 *   provide a mechanism in ToSql/FromSql to determine if a particular pg-type can decode to a
 *   given rs-type. Essentially, they allow picking a decoding mechanism based on the pg-type.
 *
 * - dbus-rs: defines a MessageItem enum which maps dbus types to rust types. All types are owned
 *   (so that they can be decoded into). This will result in higher overhead than allowing users to
 *   perform more direct convertions
 *
 * - serde: we could think about our encoding like plain-old serialization/deserialization &
 *   provide the serde/rustc-serialize interfaces into it. This is probably more work but is
 *   potentially the most convenient.
 */

use super::{MessageIter, MessageRef};
use crate::bus;
use ffi::{c_char, c_int};
use std::ffi::CStr;
use utf8_cstr::Utf8CStr;

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
    // type signature?
    // function to do append?
    // Do we need a ToOwned bit? Check ToSql
    fn to_message(&self, m: &mut MessageRef) -> crate::Result<()>;
}

/**
 * Allows types to provide a conversion from a dbus message
 *
 * NOTE: the restriction of `Self: Sized` may cause us to have less than ideal impls sometimes. We
 * may need to add a `from_message_to()` that takes a reference, much like `Clone`.
 */
pub trait FromSdBusMessage<'a> {
    fn from_message(m: &'a mut MessageIter<'a>) -> crate::Result<Option<Self>>
    where
        Self: Sized;
}

impl<T: SdBusMessageDirect> ToSdBusMessage for T {
    fn to_message(&self, m: &mut MessageRef) -> crate::Result<()> {
        unsafe { m.append_basic_raw(Self::dbus_type(), self as *const _ as *const _) }
    }
}

impl<'a, T: SdBusMessageDirect + 'a> FromSdBusMessage<'a> for T {
    fn from_message(m: &'a mut MessageIter<'a>) -> crate::Result<Option<Self>>
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
//                 sd_try!(ffi::sd_bus_message_append(m, &c_type, self as *const _));
//                 Ok(())
//             }
//         }
//
//         impl FromSdBusMessage for $typ {
//             fn from_message(m: &mut MessageRef) -> ::systemd::Result<Self> {
//                 let c_type : [u8;2] = [ $dbus_type, '\0' ];
//                 let v : Self = unsafe { uninitialized() };
//                 sd_try!(ffi::sd_bus_message_read(m, &c_type, &v));
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
    fn from_message(m: &'a mut MessageIter<'a>) -> crate::Result<Option<Self>>
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

// For string likes, (object path, string, signature) sd_bus_message_read_basic returns a *const
// c_char reference to the string owned by the underlying message. Unclear if we can represent this
// without copying.
//
// If we could use &MessageRef instead this could be useful.
impl<'a> FromSdBusMessage<'a> for &'a bus::ObjectPath {
    fn from_message(m: &'a mut MessageIter<'a>) -> crate::Result<Option<Self>>
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
    fn from_message(m: &'a mut MessageIter<'a>) -> crate::Result<Option<Self>>
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

// TODO:
//  string-likes (string, object path, signature)
//  array
//  variant
//  struct
//  dict
//
