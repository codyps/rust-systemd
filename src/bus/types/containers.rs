use super::{FromSdBusMessage, SdBusMessageDirect, ToSdBusMessage, UnixFd};
use crate::bus::{ContainerType, MessageIter, MessageRef};
use crate::{CStrArgument, Error};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::ffi::{CStr, CString};
use std::hash::{BuildHasher, Hash};
use std::io::ErrorKind;
use utf8_cstr::Utf8CStr;

/// A value's complete D-Bus type signature, including container delimiters.
/// libsystemd validates signatures when opening or entering containers.
///
/// Implementations must describe exactly one complete type. Its encoding via
/// [`ToSdBusMessage`] and decoding via [`FromSdBusMessage`] must write or read
/// exactly one value with this signature. The signature describes the Rust
/// type, so it must not depend on an individual value.
pub trait SdBusType {
    fn signature() -> String;
}

/// A basic D-Bus type that may be used as a dictionary key.
/// Its [`SdBusType::signature`] must be a single basic type code, never a
/// container. libsystemd also checks this when opening dictionary entries.
pub trait SdBusBasicType: SdBusType {}

impl<T: SdBusMessageDirect> SdBusType for T {
    fn signature() -> String {
        char::from(T::dbus_type()).to_string()
    }
}
impl<T: SdBusMessageDirect> SdBusBasicType for T {}

macro_rules! basic_type {
    ($type:ty, $signature:literal) => {
        impl SdBusType for $type {
            fn signature() -> String {
                $signature.to_owned()
            }
        }
        impl SdBusBasicType for $type {}
    };
}
basic_type!(bool, "b");
basic_type!(UnixFd, "h");
basic_type!(&Utf8CStr, "s");
basic_type!(&str, "s");
basic_type!(String, "s");
basic_type!(&String, "s");
basic_type!(&CStr, "s");
basic_type!(CString, "s");
basic_type!(&CString, "s");
basic_type!(Cow<'_, CStr>, "s");
basic_type!(Cow<'_, str>, "s");

impl ToSdBusMessage for &str {
    fn to_message(&self, message: &mut MessageRef) -> crate::Result<()> {
        let value = self.try_into_cstr()?;
        // SAFETY: the live conversion buffer contains one NUL-terminated string.
        unsafe { message.append_basic_raw(b's', value.as_ptr().cast()) }
    }
}
impl ToSdBusMessage for &CStr {
    fn to_message(&self, message: &mut MessageRef) -> crate::Result<()> {
        // SAFETY: self is a live NUL-terminated C string. libsystemd validates
        // D-Bus UTF-8 requirements and copies the value during this call.
        unsafe { message.append_basic_raw(b's', self.as_ptr().cast()) }
    }
}
impl ToSdBusMessage for CString {
    fn to_message(&self, message: &mut MessageRef) -> crate::Result<()> {
        self.as_c_str().to_message(message)
    }
}
impl ToSdBusMessage for &CString {
    fn to_message(&self, message: &mut MessageRef) -> crate::Result<()> {
        self.as_c_str().to_message(message)
    }
}
impl ToSdBusMessage for Cow<'_, CStr> {
    fn to_message(&self, message: &mut MessageRef) -> crate::Result<()> {
        self.as_ref().to_message(message)
    }
}
impl ToSdBusMessage for Cow<'_, str> {
    fn to_message(&self, message: &mut MessageRef) -> crate::Result<()> {
        self.as_ref().to_message(message)
    }
}
impl<'a> FromSdBusMessage<'a> for &'a CStr {
    fn from_message(message: &mut MessageIter<'a>) -> crate::Result<Option<Self>> {
        Ok(<&Utf8CStr>::from_message(message)?.map(AsRef::as_ref))
    }
}
impl<'a> FromSdBusMessage<'a> for CString {
    fn from_message(message: &mut MessageIter<'a>) -> crate::Result<Option<Self>> {
        Ok(<&CStr>::from_message(message)?.map(ToOwned::to_owned))
    }
}
impl ToSdBusMessage for String {
    fn to_message(&self, message: &mut MessageRef) -> crate::Result<()> {
        self.as_str().to_message(message)
    }
}
impl ToSdBusMessage for &String {
    fn to_message(&self, message: &mut MessageRef) -> crate::Result<()> {
        self.as_str().to_message(message)
    }
}
impl<'a> FromSdBusMessage<'a> for String {
    fn from_message(message: &mut MessageIter<'a>) -> crate::Result<Option<Self>> {
        Ok(<&Utf8CStr>::from_message(message)?.map(ToString::to_string))
    }
}
impl<'a> FromSdBusMessage<'a> for &'a str {
    fn from_message(message: &mut MessageIter<'a>) -> crate::Result<Option<Self>> {
        Ok(<&Utf8CStr>::from_message(message)?.map(AsRef::as_ref))
    }
}

fn write_container(
    message: &mut MessageRef,
    kind: ContainerType,
    contents: &str,
    write: impl FnOnce(&mut MessageRef) -> crate::Result<()>,
) -> crate::Result<()> {
    message.open_container(kind, contents)?;
    let result = write(message);
    let close = message.close_container();
    result.and(close)
}

// A failed decoder may leave a partially consumed container. Reset to a known
// root cursor rather than exposing that partial stack to the next typed read.
fn read_container<'a, T>(
    message: &mut MessageIter<'a>,
    kind: ContainerType,
    contents: &str,
    read: impl FnOnce(&mut MessageIter<'a>) -> crate::Result<T>,
) -> crate::Result<Option<T>> {
    if !message.enter_container(kind, contents)? {
        return Ok(None);
    }
    struct ReadGuard<'m, 'a> {
        message: &'m mut MessageIter<'a>,
        complete: bool,
    }
    impl Drop for ReadGuard<'_, '_> {
        fn drop(&mut self) {
            if !self.complete {
                let _ = self.message.rewind();
            }
        }
    }
    let mut guard = ReadGuard {
        message,
        complete: false,
    };
    read(guard.message).and_then(|value| {
        guard.message.exit_container()?;
        guard.complete = true;
        Ok(Some(value))
    })
}

fn required<'a, T: FromSdBusMessage<'a>>(message: &mut MessageIter<'a>) -> crate::Result<T> {
    T::from_message(message)?
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "missing container member"))
}

impl<T: SdBusType> SdBusType for Vec<T> {
    fn signature() -> String {
        format!("a{}", T::signature())
    }
}
impl<T: SdBusType + ToSdBusMessage> ToSdBusMessage for Vec<T> {
    fn to_message(&self, message: &mut MessageRef) -> crate::Result<()> {
        self.as_slice().to_message(message)
    }
}
impl<T: SdBusType> SdBusType for &[T] {
    fn signature() -> String {
        format!("a{}", T::signature())
    }
}
impl<T: SdBusType + ToSdBusMessage> ToSdBusMessage for &[T] {
    fn to_message(&self, message: &mut MessageRef) -> crate::Result<()> {
        write_container(message, ContainerType::Array, &T::signature(), |message| {
            for value in *self {
                value.to_message(message)?;
            }
            Ok(())
        })
    }
}
impl<'a, T: SdBusType + FromSdBusMessage<'a>> FromSdBusMessage<'a> for Vec<T> {
    fn from_message(message: &mut MessageIter<'a>) -> crate::Result<Option<Self>> {
        read_container(message, ContainerType::Array, &T::signature(), |message| {
            let mut values = Vec::new();
            while !message.at_end()? {
                values.push(required(message)?);
            }
            Ok(values)
        })
    }
}

/// One dictionary entry. D-Bus permits entries only inside an array, and requires
/// a basic key type. `Vec<DictEntry<K, V>>` preserves order and duplicate keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictEntry<K, V>(pub K, pub V);

impl<K: SdBusBasicType, V: SdBusType> SdBusType for DictEntry<K, V> {
    fn signature() -> String {
        format!("{{{}{}}}", K::signature(), V::signature())
    }
}
impl<K: SdBusBasicType + ToSdBusMessage, V: SdBusType + ToSdBusMessage> ToSdBusMessage
    for DictEntry<K, V>
{
    fn to_message(&self, message: &mut MessageRef) -> crate::Result<()> {
        let contents = format!("{}{}", K::signature(), V::signature());
        write_container(message, ContainerType::DictEntry, &contents, |message| {
            self.0.to_message(message)?;
            self.1.to_message(message)
        })
    }
}
impl<'a, K: SdBusBasicType + FromSdBusMessage<'a>, V: SdBusType + FromSdBusMessage<'a>>
    FromSdBusMessage<'a> for DictEntry<K, V>
{
    fn from_message(message: &mut MessageIter<'a>) -> crate::Result<Option<Self>> {
        let contents = format!("{}{}", K::signature(), V::signature());
        read_container(message, ContainerType::DictEntry, &contents, |message| {
            Ok(Self(required(message)?, required(message)?))
        })
    }
}

/// A variant containing a value of the expected type `T`. The outer signature is
/// always `v`; the contained signature is checked when decoding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant<T>(pub T);
impl<T> SdBusType for Variant<T> {
    fn signature() -> String {
        "v".to_owned()
    }
}
impl<T: SdBusType + ToSdBusMessage> ToSdBusMessage for Variant<T> {
    fn to_message(&self, message: &mut MessageRef) -> crate::Result<()> {
        write_container(
            message,
            ContainerType::Variant,
            &T::signature(),
            |message| self.0.to_message(message),
        )
    }
}
impl<'a, T: SdBusType + FromSdBusMessage<'a>> FromSdBusMessage<'a> for Variant<T> {
    fn from_message(message: &mut MessageIter<'a>) -> crate::Result<Option<Self>> {
        read_container(
            message,
            ContainerType::Variant,
            &T::signature(),
            |message| Ok(Self(required(message)?)),
        )
    }
}

macro_rules! tuple {
    ($($type:ident : $index:tt),+) => {
        impl<$($type: SdBusType),+> SdBusType for ($($type,)+) {
            fn signature() -> String {
                let mut signature = String::from("(");
                $(signature.push_str(&$type::signature());)+
                signature.push(')');
                signature
            }
        }
        impl<$($type: SdBusType + ToSdBusMessage),+> ToSdBusMessage for ($($type,)+) {
            fn to_message(&self, message: &mut MessageRef) -> crate::Result<()> {
                let signature = Self::signature();
                write_container(message, ContainerType::Struct, &signature[1..signature.len()-1], |message| {
                    $(self.$index.to_message(message)?;)+
                    Ok(())
                })
            }
        }
        impl<'a, $($type: SdBusType + FromSdBusMessage<'a>),+> FromSdBusMessage<'a> for ($($type,)+) {
            fn from_message(message: &mut MessageIter<'a>) -> crate::Result<Option<Self>> {
                let signature = Self::signature();
                read_container(message, ContainerType::Struct, &signature[1..signature.len()-1], |message| {
                    Ok(($(required::<$type>(message)?,)+))
                })
            }
        }
    };
}
tuple!(A:0);
tuple!(A:0, B:1);
tuple!(A:0, B:1, C:2);
tuple!(A:0, B:1, C:2, D:3);
tuple!(A:0, B:1, C:2, D:3, E:4);
tuple!(A:0, B:1, C:2, D:3, E:4, F:5);
tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6);
tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7);
tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8);
tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9);
tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10);
tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11);

impl<K: SdBusBasicType, V: SdBusType> SdBusType for BTreeMap<K, V> {
    fn signature() -> String {
        format!("a{{{}{}}}", K::signature(), V::signature())
    }
}
impl<K: SdBusBasicType + ToSdBusMessage, V: SdBusType + ToSdBusMessage> ToSdBusMessage
    for BTreeMap<K, V>
{
    fn to_message(&self, message: &mut MessageRef) -> crate::Result<()> {
        let entry = DictEntry::<K, V>::signature();
        write_container(message, ContainerType::Array, &entry, |message| {
            for (key, value) in self {
                write_container(
                    message,
                    ContainerType::DictEntry,
                    &entry[1..entry.len() - 1],
                    |message| {
                        key.to_message(message)?;
                        value.to_message(message)
                    },
                )?;
            }
            Ok(())
        })
    }
}
impl<'a, K: SdBusBasicType + FromSdBusMessage<'a> + Ord, V: SdBusType + FromSdBusMessage<'a>>
    FromSdBusMessage<'a> for BTreeMap<K, V>
{
    fn from_message(message: &mut MessageIter<'a>) -> crate::Result<Option<Self>> {
        Vec::<DictEntry<K, V>>::from_message(message)?
            .map(|entries| {
                let mut map = BTreeMap::new();
                for DictEntry(key, value) in entries {
                    if map.insert(key, value).is_some() {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            "duplicate dictionary key",
                        ));
                    }
                }
                Ok(map)
            })
            .transpose()
    }
}

impl<K: SdBusBasicType, V: SdBusType, S> SdBusType for HashMap<K, V, S> {
    fn signature() -> String {
        BTreeMap::<K, V>::signature()
    }
}
impl<K: SdBusBasicType + ToSdBusMessage, V: SdBusType + ToSdBusMessage, S> ToSdBusMessage
    for HashMap<K, V, S>
{
    fn to_message(&self, message: &mut MessageRef) -> crate::Result<()> {
        let entry = DictEntry::<K, V>::signature();
        write_container(message, ContainerType::Array, &entry, |message| {
            for (key, value) in self {
                write_container(
                    message,
                    ContainerType::DictEntry,
                    &entry[1..entry.len() - 1],
                    |message| {
                        key.to_message(message)?;
                        value.to_message(message)
                    },
                )?;
            }
            Ok(())
        })
    }
}
impl<
        'a,
        K: SdBusBasicType + FromSdBusMessage<'a> + Eq + Hash,
        V: SdBusType + FromSdBusMessage<'a>,
        S: BuildHasher + Default,
    > FromSdBusMessage<'a> for HashMap<K, V, S>
{
    fn from_message(message: &mut MessageIter<'a>) -> crate::Result<Option<Self>> {
        Vec::<DictEntry<K, V>>::from_message(message)?
            .map(|entries| {
                let mut map = HashMap::with_hasher(S::default());
                for DictEntry(key, value) in entries {
                    if map.insert(key, value).is_some() {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            "duplicate dictionary key",
                        ));
                    }
                }
                Ok(map)
            })
            .transpose()
    }
}
