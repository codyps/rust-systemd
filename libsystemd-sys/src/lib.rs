//! Contains definitions for low-level bindings.
//!
//! Most of this module is Rust versions of the systemd headers. The goal of
//! this crate is to make it unattractive to ever use the FFI directly, but
//! it's there if you need it.
//!
//! Items in this module corresponding to systemd functions are well-documented
//! by the systemd man pages.

extern crate libc;
pub use libc::{size_t, pid_t, uid_t, gid_t, signalfd_siginfo, siginfo_t, clockid_t, int64_t,
               uint32_t, uint64_t};
pub use std::os::raw::{c_char, c_int, c_void, c_uint};

pub mod id128;
pub mod event;
pub mod daemon;
pub mod journal;
pub mod login;

#[repr(C)]
pub struct iovec {
    pub iov_base: *mut c_void,
    pub iov_len: size_t,
}

#[repr(C)]
pub struct const_iovec {
    pub iov_base: *const c_void,
    pub iov_len: size_t,
}

pub fn array_to_iovecs(args: &[&str]) -> Vec<const_iovec> {
    args.iter()
        .map(|d| {
            const_iovec {
                iov_base: d.as_ptr() as *const c_void,
                iov_len: d.len() as size_t,
            }
        })
        .collect()
}

#[cfg(feature = "bus")]
pub mod bus;
