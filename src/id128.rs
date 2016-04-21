use std::mem::uninitialized;
use std::fmt;
use ffi;
use std::ffi::CStr;
use super::Result;

pub struct Id128 {
    inner: ffi::id128::sd_id128_t,
}

impl fmt::Display for Id128 {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        for b in self.inner.bytes.iter() {
            try!(write!(fmt, "{:02x}", b));
        }
        Ok(())
    }
}

impl Id128 {
    pub fn from_cstr(s: &CStr) -> Result<Id128> {
        let mut r: Id128 = unsafe { uninitialized() };
        sd_try!(ffi::id128::sd_id128_from_string(s.as_ptr(), &mut r.inner));
        Ok(r)
    }

    pub fn from_random() -> Result<Id128> {
        let mut r: Id128 = unsafe { uninitialized() };
        sd_try!(ffi::id128::sd_id128_randomize(&mut r.inner));
        Ok(r)
    }

    pub fn from_machine() -> Result<Id128> {
        let mut r: Id128 = unsafe { uninitialized() };
        sd_try!(ffi::id128::sd_id128_get_machine(&mut r.inner));
        Ok(r)
    }

    pub fn from_boot() -> Result<Id128> {
        let mut r: Id128 = unsafe { uninitialized() };
        sd_try!(ffi::id128::sd_id128_get_boot(&mut r.inner));
        Ok(r)
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.inner.bytes
    }
}
