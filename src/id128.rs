//! APIs to process and generate 128-bit ID values for systemd.
//!
//! These ID values are a generalization of OSF UUIDs but use a
//! simpler string format. See `man 3 sd-id128` for more details.

use super::Result;
use crate::ffi_result;
use std::ffi::CStr;
use std::fmt;

/// A 128-bit ID for systemd.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id128 {
    pub(crate) inner: ffi::id128::sd_id128_t,
}

impl fmt::Debug for Id128 {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "Id128 {{ ")?;
        <Self as fmt::Display>::fmt(self, fmt)?;
        write!(fmt, " }}")
    }
}

impl fmt::Display for Id128 {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in self.inner.bytes.iter() {
            write!(fmt, "{b:02x}")?;
        }
        Ok(())
    }
}

impl Default for Id128 {
    /// Return a null-ID, consisting of only NUL bytes.
    fn default() -> Self {
        Id128 {
            inner: ffi::id128::sd_id128_t { bytes: [0x00; 16] },
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Id128 {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Id128 {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let cstr: Box<CStr> = serde::Deserialize::deserialize(deserializer)?;
        Id128::from_cstr(&cstr).map_err(serde::de::Error::custom)
    }
}

impl Id128 {
    pub fn from_cstr(s: &CStr) -> Result<Id128> {
        let mut r = Id128::default();
        ffi_result(unsafe { ffi::id128::sd_id128_from_string(s.as_ptr(), &mut r.inner) })?;
        Ok(r)
    }

    pub fn from_random() -> Result<Id128> {
        let mut r = Id128::default();
        ffi_result(unsafe { ffi::id128::sd_id128_randomize(&mut r.inner) })?;
        Ok(r)
    }

    pub fn from_machine() -> Result<Id128> {
        let mut r = Id128::default();
        ffi_result(unsafe { ffi::id128::sd_id128_get_machine(&mut r.inner) })?;
        Ok(r)
    }

    pub fn from_machine_app_specific(app_id: &Id128) -> Result<Id128> {
        let mut r = Id128::default();
        ffi_result(unsafe {
            ffi::id128::sd_id128_get_machine_app_specific(*app_id.as_raw(), &mut r.inner)
        })?;
        Ok(r)
    }

    pub fn from_boot() -> Result<Id128> {
        let mut r = Id128::default();
        ffi_result(unsafe { ffi::id128::sd_id128_get_boot(&mut r.inner) })?;
        Ok(r)
    }

    pub fn from_boot_app_specific(app_id: &Id128) -> Result<Id128> {
        let mut r = Id128::default();
        ffi_result(unsafe {
            ffi::id128::sd_id128_get_boot_app_specific(*app_id.as_raw(), &mut r.inner)
        })?;
        Ok(r)
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.inner.bytes
    }

    pub fn as_raw(&self) -> &ffi::id128::sd_id128_t {
        &self.inner
    }

    pub fn as_raw_mut(&mut self) -> &mut ffi::id128::sd_id128_t {
        &mut self.inner
    }
}
