//! APIs to process and generate 128-bit ID values for systemd.
//!
//! These ID values are a generalization of OSF UUIDs but use a
//! simpler string format. See `man 3 sd-id128` for more details.

use super::Result;
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
            write!(fmt, "{:02x}", b)?;
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

impl Id128 {
    pub fn from_cstr(s: &CStr) -> Result<Id128> {
        let mut r = Id128::default();
        sd_try!(ffi::id128::sd_id128_from_string(s.as_ptr(), &mut r.inner));
        Ok(r)
    }

    pub fn from_random() -> Result<Id128> {
        let mut r = Id128::default();
        sd_try!(ffi::id128::sd_id128_randomize(&mut r.inner));
        Ok(r)
    }

    pub fn from_machine() -> Result<Id128> {
        let mut r = Id128::default();
        sd_try!(ffi::id128::sd_id128_get_machine(&mut r.inner));
        Ok(r)
    }

    pub fn from_boot() -> Result<Id128> {
        let mut r = Id128::default();
        sd_try!(ffi::id128::sd_id128_get_boot(&mut r.inner));
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
