use super::types::{FromSdBusMessage, SdBusBasicType, SdBusType, ToSdBusMessage};
use super::{BusName, InterfaceName, MemberName, ObjectPath};
use crate::CStrArgument;
use std::borrow::{Borrow, Cow};
use std::convert::TryFrom;
use std::ffi::{CStr, CString};
use std::ops::Deref;

macro_rules! checked_name {
    ($name:ident, $owned:ident, $wire:literal) => {
        #[doc = concat!("Owned, validated storage for [`", stringify!($name), "`].")]
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $owned {
            inner: CString,
        }

        impl $name {
            /// Validate a convertible string, borrowing C strings where possible.
            /// A Rust string without a terminator becomes owned storage. Passing
            /// an owned C string transfers its buffer without copying.
            /// Invalid values return `InvalidInput`.
            #[allow(clippy::new_ret_no_self)] // Self is a borrowed, unsized string type.
            pub fn new<'a>(value: impl CStrArgument<'a>) -> crate::Result<Cow<'a, Self>> {
                match value.try_into_cstr()? {
                    Cow::Borrowed(value) => {
                        Self::from_bytes(value.to_bytes_with_nul()).map(Cow::Borrowed)
                    }
                    Cow::Owned(value) => {
                        Self::from_bytes(value.as_bytes_with_nul())?;
                        Ok(Cow::Owned($owned { inner: value }))
                    }
                }
            }
        }

        impl $owned {
            /// Release the validated wrapper, preserving its owned buffer.
            pub fn into_c_string(self) -> CString {
                self.inner
            }
        }

        impl Deref for $owned {
            type Target = $name;
            fn deref(&self) -> &Self::Target {
                // SAFETY: all construction paths validate the immutable buffer.
                unsafe { $name::from_bytes_unchecked(self.inner.as_bytes_with_nul()) }
            }
        }
        impl AsRef<$name> for $owned {
            fn as_ref(&self) -> &$name {
                self
            }
        }
        impl AsRef<CStr> for $owned {
            fn as_ref(&self) -> &CStr {
                &self.inner
            }
        }
        impl Borrow<$name> for $owned {
            fn borrow(&self) -> &$name {
                self
            }
        }
        impl ToOwned for $name {
            type Owned = $owned;
            fn to_owned(&self) -> $owned {
                $owned {
                    inner: self.deref().to_owned(),
                }
            }
        }
        impl AsRef<CStr> for $name {
            fn as_ref(&self) -> &CStr {
                self
            }
        }
        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.deref() == other.deref()
            }
        }
        impl Eq for $name {}
        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }
        impl Ord for $name {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.deref().cmp(other.deref())
            }
        }
        impl std::hash::Hash for $name {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                self.deref().hash(state)
            }
        }
        impl From<&$name> for $owned {
            fn from(value: &$name) -> Self {
                value.to_owned()
            }
        }
        impl From<$owned> for CString {
            fn from(value: $owned) -> Self {
                value.into_c_string()
            }
        }
        impl std::str::FromStr for $owned {
            type Err = std::io::Error;
            fn from_str(value: &str) -> crate::Result<Self> {
                Ok($name::new(value)?.into_owned())
            }
        }
        impl TryFrom<CString> for $owned {
            type Error = std::io::Error;
            fn try_from(value: CString) -> crate::Result<Self> {
                Ok($name::new(value)?.into_owned())
            }
        }
        impl TryFrom<String> for $owned {
            type Error = std::io::Error;
            fn try_from(value: String) -> crate::Result<Self> {
                Ok($name::new(value)?.into_owned())
            }
        }
        impl TryFrom<&CStr> for $owned {
            type Error = std::io::Error;
            fn try_from(value: &CStr) -> crate::Result<Self> {
                Ok($name::new(value)?.into_owned())
            }
        }
        impl TryFrom<&str> for $owned {
            type Error = std::io::Error;
            fn try_from(value: &str) -> crate::Result<Self> {
                value.parse()
            }
        }
        impl TryFrom<&String> for $owned {
            type Error = std::io::Error;
            fn try_from(value: &String) -> crate::Result<Self> {
                Ok($name::new(value)?.into_owned())
            }
        }
        impl TryFrom<&CString> for $owned {
            type Error = std::io::Error;
            fn try_from(value: &CString) -> crate::Result<Self> {
                Ok($name::new(value)?.into_owned())
            }
        }
        impl<'a> From<Cow<'a, $name>> for $owned {
            fn from(value: Cow<'a, $name>) -> Self {
                value.into_owned()
            }
        }
        impl<'a> CStrArgument<'a> for $owned {
            fn try_into_cstr(self) -> crate::Result<Cow<'a, CStr>> {
                Ok(Cow::Owned(self.inner))
            }
        }
        impl<'a> CStrArgument<'a> for &'a $owned {
            fn try_into_cstr(self) -> crate::Result<Cow<'a, CStr>> {
                Ok(Cow::Borrowed(&self.inner))
            }
        }
        impl<'a> CStrArgument<'a> for Cow<'a, $name> {
            fn try_into_cstr(self) -> crate::Result<Cow<'a, CStr>> {
                match self {
                    Cow::Borrowed(value) => Ok(Cow::Borrowed(value)),
                    Cow::Owned(value) => Ok(Cow::Owned(value.inner)),
                }
            }
        }
        impl<'a> CStrArgument<'a> for &'a Cow<'_, $name> {
            fn try_into_cstr(self) -> crate::Result<Cow<'a, CStr>> {
                Ok(Cow::Borrowed(self.as_ref()))
            }
        }

        impl SdBusType for &$name {
            fn signature() -> String {
                char::from($wire).to_string()
            }
        }
        impl SdBusBasicType for &$name {}
        impl SdBusType for $owned {
            fn signature() -> String {
                <&$name>::signature()
            }
        }
        impl SdBusBasicType for $owned {}
        impl SdBusType for &$owned {
            fn signature() -> String {
                <&$name>::signature()
            }
        }
        impl SdBusBasicType for &$owned {}
        impl SdBusType for Cow<'_, $name> {
            fn signature() -> String {
                <&$name>::signature()
            }
        }
        impl SdBusBasicType for Cow<'_, $name> {}
        impl SdBusType for &Cow<'_, $name> {
            fn signature() -> String {
                <&$name>::signature()
            }
        }
        impl SdBusBasicType for &Cow<'_, $name> {}

        impl ToSdBusMessage for &$name {
            fn to_message(&self, message: &mut super::MessageRef) -> crate::Result<()> {
                // SAFETY: validation guarantees a live NUL-terminated value
                // matching the native string/object-path type.
                unsafe { message.append_basic_raw($wire, self.as_ptr().cast()) }
            }
        }
        impl ToSdBusMessage for $owned {
            fn to_message(&self, message: &mut super::MessageRef) -> crate::Result<()> {
                let name: &$name = self;
                name.to_message(message)
            }
        }
        impl ToSdBusMessage for &$owned {
            fn to_message(&self, message: &mut super::MessageRef) -> crate::Result<()> {
                let name: &$name = self;
                name.to_message(message)
            }
        }
        impl ToSdBusMessage for Cow<'_, $name> {
            fn to_message(&self, message: &mut super::MessageRef) -> crate::Result<()> {
                self.as_ref().to_message(message)
            }
        }
        impl ToSdBusMessage for &Cow<'_, $name> {
            fn to_message(&self, message: &mut super::MessageRef) -> crate::Result<()> {
                self.as_ref().to_message(message)
            }
        }
        impl<'a> FromSdBusMessage<'a> for &'a $name {
            fn from_message(message: &mut super::MessageIter<'a>) -> crate::Result<Option<Self>> {
                // SAFETY: these native types return pointers into the sealed
                // body. The body remains live for the message borrow 'a.
                let value: Option<&'a CStr> = unsafe {
                    message.read_basic_raw($wire, |ptr: *const ffi::c_char| CStr::from_ptr(ptr))?
                };
                value
                    .map(|value| {
                        $name::from_bytes(value.to_bytes_with_nul()).map_err(|error| {
                            std::io::Error::new(std::io::ErrorKind::InvalidData, error)
                        })
                    })
                    .transpose()
            }
        }
        impl<'a> FromSdBusMessage<'a> for $owned {
            fn from_message(message: &mut super::MessageIter<'a>) -> crate::Result<Option<Self>> {
                Ok(<&$name>::from_message(message)?.map(ToOwned::to_owned))
            }
        }
        impl<'a> FromSdBusMessage<'a> for Cow<'a, $name> {
            fn from_message(message: &mut super::MessageIter<'a>) -> crate::Result<Option<Self>> {
                Ok(<&$name>::from_message(message)?.map(Cow::Borrowed))
            }
        }
    };
}

checked_name!(ObjectPath, ObjectPathBuf, b'o');
checked_name!(InterfaceName, InterfaceNameBuf, b's');
checked_name!(BusName, BusNameBuf, b's');
checked_name!(MemberName, MemberNameBuf, b's');
