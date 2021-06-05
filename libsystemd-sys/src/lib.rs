//! Low-level bindings to libsystemd (and similar) libraries
//!
//! Items in this module correspond to systemd functions/types that are documented by the systemd
//! (`sd_*`) man pages.

#![warn(rust_2018_idioms)]

pub use libc::{clockid_t, gid_t, iovec, pid_t, siginfo_t, signalfd_siginfo, size_t, uid_t};
pub use std::os::raw::{c_char, c_int, c_uint, c_void};

pub mod daemon;
pub mod event;
pub mod id128;
#[cfg(feature = "journal")]
pub mod journal;
pub mod login;

/// Helper type to mark functions systemd functions that promise not to modify the underlying iovec
/// data.  There is no corresponding type in libc, so their function signatures take *const iovec,
/// which technically allow iov_base to be modified.  However, ConstIovec provides the same ABI, so
/// it can be used to make the function interface easier to work with.
#[repr(C)]
pub struct ConstIovec {
    pub iov_base: *const c_void,
    pub iov_len: size_t,
}

impl ConstIovec {
    ///
    /// # Safety
    ///
    /// Lifetime of `arg` must be long enough to cover future dereferences of the internal
    /// `Self::iov_base` pointer.
    pub unsafe fn from_str<T>(arg: T) -> Self
    where
        T: AsRef<str>,
    {
        ConstIovec {
            iov_base: arg.as_ref().as_ptr() as *const c_void,
            iov_len: arg.as_ref().len() as size_t,
        }
    }
}

#[cfg(feature = "bus")]
pub mod bus;
