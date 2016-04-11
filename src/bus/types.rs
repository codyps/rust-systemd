use super::MessageRef;
use super::super::ffi;
use std::mem::{uninitialized};
use ffi::{c_int,c_char};

unsafe trait SdBusMessageDirect {
    fn dbus_type() -> u8;
}

trait ToSdBusMessage {
    // type signature?
    // function to do append?
    // Do we need a ToOwned bit? Check ToSql
    fn to_message(&self, m: &mut MessageRef) -> ::Result<()>;
}

trait FromSdBusMessage {
    fn from_message(m: &mut MessageRef) -> ::Result<Self>
        where Self: Sized;
}

impl<T: SdBusMessageDirect> ToSdBusMessage for T {
    fn to_message(&self, m: &mut MessageRef) -> ::Result<()> {
        sd_try!(ffi::bus::sd_bus_message_append_basic(m.as_mut_ptr(), T::dbus_type() as c_char, self as *const _ as *const _));
        Ok(())
    }
}

impl<T: SdBusMessageDirect> FromSdBusMessage for T {
    fn from_message(m: &mut MessageRef) -> ::Result<Self> {
        let mut v : Self = unsafe { uninitialized() };
        sd_try!(ffi::bus::sd_bus_message_read_basic(m.as_mut_ptr(), T::dbus_type() as c_char, &mut v as *mut _ as *mut _));
        Ok(v)
    }
}

/*
macro_rules! msg_basic {
    ($typ:ty $dbus_type:expr) => {
        impl ToSdBusMessage for $typ {
            fn to_message(&self, m: &mut MessageRef) -> ::systemd::Result<()> {
                let c_type : [u8;2] = [ $dbus_type, '\0' ];
                sd_try!(ffi::sd_bus_message_append(m, &c_type, self as *const _));
                Ok(())
            }
        }

        impl FromSdBusMessage for $typ {
            fn from_message(m: &mut MessageRef) -> ::systemd::Result<Self> {
                let c_type : [u8;2] = [ $dbus_type, '\0' ];
                let v : Self = unsafe { uninitialized() };
                sd_try!(ffi::sd_bus_message_read(m, &c_type, &v));
                Ok(v)
            }
        }
    }

    ($typ:ty $dbus_type:expr , $($rest:tt)* ) => {
        msg_basic!($typ $dbus_type);
        msg_basic!($($rest)*);
    }
}
*/

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


msg_basic!{
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
    fn to_message(&self, m: &mut MessageRef) -> ::Result<()> {
        let i : c_int = if *self { 1 } else { 0 };
        sd_try!(ffi::bus::sd_bus_message_append_basic(m.as_mut_ptr(), b'b' as c_char, &i as *const _ as *const _));
        Ok(())
    }
}

impl FromSdBusMessage for bool {
    fn from_message(m: &mut MessageRef) -> ::Result<Self> {
        let mut i : c_int = unsafe { uninitialized() };
        sd_try!(ffi::bus::sd_bus_message_read_basic(m.as_mut_ptr(), b'b' as c_char, &mut i as *mut _ as *mut _));
        Ok(i != 0)
    }
}

pub struct UnixFd(pub c_int);

impl ToSdBusMessage for UnixFd {
    fn to_message(&self, m: &mut MessageRef) -> ::Result<()> {
        let i : c_int = self.0;
        sd_try!(ffi::bus::sd_bus_message_append_basic(m.as_mut_ptr(), b'h' as c_char, &i as *const _ as *const _));
        Ok(())
    }
}

impl FromSdBusMessage for UnixFd {
    fn from_message(m: &mut MessageRef) -> ::Result<Self> {
        let mut i : c_int = unsafe { uninitialized() };
        sd_try!(ffi::bus::sd_bus_message_read_basic(m.as_mut_ptr(), b'h' as c_char, &mut i as *mut _ as *mut _));
        Ok(UnixFd(i))
    }
}

impl<'a> ToSdBusMessage for super::ObjectPath<'a> {
    fn to_message(&self, m: &mut MessageRef) -> ::Result<()> {
        sd_try!(ffi::bus::sd_bus_message_append_basic(m.as_mut_ptr(), b'o' as c_char, self.as_ptr() as *const _));
        Ok(())
    }
}

// For string likes, (object path, string, signature) sd_bus_message_read_basic returns a *const
// c_char reference to the string owned by the underlying message. Unclear if we can represent this
// without copying.
//
// If we could use &MessageRef instead this could be useful.
impl<'a> FromSdBusMessage for super::ObjectPath<'a> {
    fn from_message(m: &mut MessageRef) -> ::Result<Self> {
        let mut i : *const c_char = unsafe { uninitialized() };
        sd_try!(ffi::bus::sd_bus_message_read_basic(m.as_mut_ptr(), b'o' as c_char, &mut i as *mut _ as *mut _));
        Ok(unsafe {super::ObjectPath::from_ptr_unchecked(i)})
    }
}


/* TODO:
 *  string-likes (string, object path, signature)
 *  array
 *  variant
 *  struct
 *  dict
 */
