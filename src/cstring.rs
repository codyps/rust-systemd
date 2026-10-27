use std::borrow::Cow;
use std::ffi::{CStr, CString};

/// A string argument for an API that needs a NUL-terminated string.
///
/// Accepts Rust strings, C strings, their owned forms, `Cow`, and the crate's
/// validated bus names. Borrowed C strings (and Rust strings already ending in
/// one NUL) need no allocation for temporary calls. Owned C strings transfer
/// their buffers; owned Rust strings reuse their storage where possible.
/// APIs that retain strings copy borrowed storage into owned strings.
///
/// An interior NUL returns `InvalidInput`. A single trailing NUL is accepted.
/// The conversion does not validate UTF-8 or API-specific name/signature rules;
/// the receiving API performs those checks when required.
///
/// ```
/// use std::borrow::Cow;
/// use std::ffi::CString;
/// use systemd::CStrArgument;
/// let owned = CString::new("example").unwrap();
/// let converted = owned.as_c_str().try_into_cstr().unwrap();
/// assert!(matches!(converted, Cow::Borrowed(_)));
/// assert_eq!(converted.as_ptr(), owned.as_ptr());
/// assert!("bad\0argument".try_into_cstr().is_err());
/// ```
pub trait CStrArgument<'a>: Sized {
    /// Convert into borrowed or owned C-string storage. The returned buffer
    /// must stay alive until the native call has finished using its pointer.
    fn try_into_cstr(self) -> crate::Result<Cow<'a, CStr>>;
}

fn invalid(error: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, error.to_string())
}

fn borrowed(bytes: &[u8]) -> crate::Result<Cow<'_, CStr>> {
    if bytes.last() == Some(&0) {
        CStr::from_bytes_with_nul(bytes)
            .map(Cow::Borrowed)
            .map_err(invalid)
    } else {
        CString::new(bytes).map(Cow::Owned).map_err(invalid)
    }
}

fn owned(bytes: Vec<u8>) -> crate::Result<CString> {
    if bytes.last() == Some(&0) {
        CString::from_vec_with_nul(bytes).map_err(invalid)
    } else {
        CString::new(bytes).map_err(invalid)
    }
}

impl<'a> CStrArgument<'a> for &'a str {
    fn try_into_cstr(self) -> crate::Result<Cow<'a, CStr>> {
        borrowed(self.as_bytes())
    }
}
impl<'a> CStrArgument<'a> for &'a String {
    fn try_into_cstr(self) -> crate::Result<Cow<'a, CStr>> {
        borrowed(self.as_bytes())
    }
}
impl<'a> CStrArgument<'a> for String {
    fn try_into_cstr(self) -> crate::Result<Cow<'a, CStr>> {
        owned(self.into_bytes()).map(Cow::Owned)
    }
}
impl<'a> CStrArgument<'a> for &'a CStr {
    fn try_into_cstr(self) -> crate::Result<Cow<'a, CStr>> {
        Ok(Cow::Borrowed(self))
    }
}
impl<'a> CStrArgument<'a> for &'a CString {
    fn try_into_cstr(self) -> crate::Result<Cow<'a, CStr>> {
        Ok(Cow::Borrowed(self))
    }
}
impl<'a> CStrArgument<'a> for CString {
    fn try_into_cstr(self) -> crate::Result<Cow<'a, CStr>> {
        Ok(Cow::Owned(self))
    }
}
impl<'a> CStrArgument<'a> for Box<CStr> {
    fn try_into_cstr(self) -> crate::Result<Cow<'a, CStr>> {
        Ok(Cow::Owned(self.into()))
    }
}
impl<'a> CStrArgument<'a> for &'a Box<CStr> {
    fn try_into_cstr(self) -> crate::Result<Cow<'a, CStr>> {
        Ok(Cow::Borrowed(self))
    }
}
impl<'a> CStrArgument<'a> for &'a [u8] {
    fn try_into_cstr(self) -> crate::Result<Cow<'a, CStr>> {
        borrowed(self)
    }
}
impl<'a> CStrArgument<'a> for &'a Vec<u8> {
    fn try_into_cstr(self) -> crate::Result<Cow<'a, CStr>> {
        borrowed(self)
    }
}
impl<'a> CStrArgument<'a> for Vec<u8> {
    fn try_into_cstr(self) -> crate::Result<Cow<'a, CStr>> {
        owned(self).map(Cow::Owned)
    }
}

impl<'a> CStrArgument<'a> for &'a utf8_cstr::Utf8CStr {
    fn try_into_cstr(self) -> crate::Result<Cow<'a, CStr>> {
        Ok(Cow::Borrowed(self.as_ref()))
    }
}
impl<'a> CStrArgument<'a> for Cow<'a, CStr> {
    fn try_into_cstr(self) -> crate::Result<Cow<'a, CStr>> {
        Ok(self)
    }
}
impl<'a> CStrArgument<'a> for &'a Cow<'_, CStr> {
    fn try_into_cstr(self) -> crate::Result<Cow<'a, CStr>> {
        Ok(Cow::Borrowed(self.as_ref()))
    }
}
impl<'a> CStrArgument<'a> for Cow<'a, str> {
    fn try_into_cstr(self) -> crate::Result<Cow<'a, CStr>> {
        match self {
            Cow::Borrowed(value) => value.try_into_cstr(),
            Cow::Owned(value) => value.try_into_cstr(),
        }
    }
}
impl<'a> CStrArgument<'a> for &'a Cow<'_, str> {
    fn try_into_cstr(self) -> crate::Result<Cow<'a, CStr>> {
        self.as_ref().try_into_cstr()
    }
}
