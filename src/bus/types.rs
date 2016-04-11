use super::MessageRef;
use super::super::ffi;
use std::mem::{uninitialized};

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
        let c_type : [u8;2] = [ T::dbus_type(), b'\0' ];
        sd_try!(ffi::bus::sd_bus_message_append(m.as_mut_ptr(), &c_type as *const _ as *const _, self as *const _));
        Ok(())
    }
}

impl<T: SdBusMessageDirect> FromSdBusMessage for T {
    fn from_message(m: &mut MessageRef) -> ::Result<Self> {
        let c_type : [u8;2] = [ T::dbus_type(), b'\0' ];
        let v : Self = unsafe { uninitialized() };
        sd_try!(ffi::bus::sd_bus_message_read(m.as_mut_ptr(), &c_type as *const _ as *const _, &v));
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

/* TODO:
 *  string-likes (string, object path, signature)
 *  array
 *  boolean
 *  fd
 *  variant
 *  struct
 *  dict
 *
 *
 */
