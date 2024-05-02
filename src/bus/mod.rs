// Persistent issues:
//
//  - callbacks trigger allocation
//    The underlying systemd calls which add callbacks perform allocation internally (they allocate
//    a `slot`). We add our own `Box` allocation. This currently isn't exposed to users of the
//    methods: they pass a `Fn`, not a `Box<Fn>`.
//
//    In some cases, this restriction is probably not necessary, but it's unclear how to handle.
//
//  - very easy to create multiple mutable references to the same data
//    The messages, slots, bus, etc all have methods to obtain the other end of the "link".
//    Messages can get the bus they're attached to. They're then able to upgrade their ref to a
//    owned reference, and can then use that owned reference as mutable.
//
//    We may just need to restrict the ability to adjust ownership and obtain references to less
//    than what is possible with sd-bus directly.

//use enumflags2_derive::EnumFlags;
use ffi::{c_char, c_int, c_void, pid_t};
use foreign_types::{foreign_type, ForeignType, ForeignTypeRef};
use std::ffi::CStr;
use std::marker::PhantomData;
use std::mem::{forget, MaybeUninit};
use std::ops::Deref;
use std::os::unix::io::AsRawFd;
use std::ptr;
use std::result;
use std::time::Duration;
use std::{fmt, str};

use super::usec_from_duration;
use utf8_cstr::Utf8CStr;

pub mod types;

/**
 * Result type for dbus calls that contains errors returned by remote services (and local errors as
 * well).
 *
 * For functions that can pass over dbus, sd-bus provides detailed error information for all
 * failures, including those cause by bus failures (not necessarily errors sent by the called
 * method).
 *
 * To clarify: getting this error does not necessarily mean it comes from a remote service. It
 * might be a local failure (resource exhaustion, programmer error, service unreachable) as well.
 */
pub type Result<T> = result::Result<T, Error>;

/**
 * A wrapper which promises it always holds a valid dbus object path
 *
 * Requirements (from dbus spec 0.26):
 *
 * - path must begin with ASCII '/' and consist of elements separated by slash characters
 * - each element must only contain the ASCII characters '[A-Z][a-z][0-9]_'
 * - No element may be the empty string
 * - Multiple '/' characters may not occur in sequence
 * - A trailing '/' character is not allowed unless the path is the root path
 * - Further, sd-bus additionally requires nul ('\0') termination of paths.
 */
#[derive(Debug)]
pub struct ObjectPath {
    inner: CStr,
}

impl ObjectPath {
    /**
     * Create a path reference from a u8 slice. Performs all checking needed to ensure requirements
     * are met.
     */
    pub fn from_bytes(b: &[u8]) -> result::Result<&ObjectPath, &'static str> {
        if b.is_empty() {
            return Err("Path must have at least 1 character ('/')");
        }

        if b[0] != b'/' {
            return Err("Path must begin with '/'");
        }

        for w in b.windows(2) {
            let prev = w[0];
            let c = w[1];

            match c {
                b'/' => {
                    if prev == b'/' {
                        return Err("Path must not have 2 '/' next to each other");
                    }
                }
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' => {
                    // Ok
                }
                b'\0' => {
                    if prev == b'/' && b.len() != 2 {
                        return Err("Path must not end in '/' unless it is the root path");
                    }

                    return Ok(unsafe { ObjectPath::from_bytes_unchecked(b) });
                }
                _ => {
                    return Err("Invalid character in path, only '[A-Z][a-z][0-9]_/' allowed");
                }
            }
        }

        Err("Path must be terminated in a '\\0' byte (for use by sd-bus)")
    }

    /// # Safety
    ///
    /// - `b` must be nul (`'\0'`) terminated
    /// - `b` must be a valid object path string
    #[inline]
    pub unsafe fn from_bytes_unchecked(b: &[u8]) -> &ObjectPath {
        &*(b as *const [u8] as *const ObjectPath)
    }

    /// # Safety
    ///
    /// - `b` must have a lifetime of at least `'b`
    /// - `b` must be nul (`'\0'`) terminated
    /// - `b` must be a valid object path string
    #[inline]
    pub unsafe fn from_ptr_unchecked<'b>(b: *const c_char) -> &'b ObjectPath {
        Self::from_bytes_unchecked(CStr::from_ptr(b).to_bytes())
    }
}

impl Deref for ObjectPath {
    type Target = CStr;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[test]
fn t_path() {
    ObjectPath::from_bytes(b"/\0").unwrap();
    ObjectPath::from_bytes(b"\0").err().unwrap();
    ObjectPath::from_bytes(b"/").err().unwrap();
    ObjectPath::from_bytes(b"/h\0").unwrap();
    ObjectPath::from_bytes(b"/hello\0").unwrap();
    ObjectPath::from_bytes(b"/hello/\0").err().unwrap();
    ObjectPath::from_bytes(b"/hello/goodbye/013/4/HA\0").unwrap();
    ObjectPath::from_bytes(b"/hello/goodbye/013/4?/HA\0")
        .err()
        .unwrap();
}

/**
 * A wrapper which promises it always holds a validated dbus interface name
 */
#[derive(Debug)]
pub struct InterfaceName {
    inner: CStr,
}

impl InterfaceName {
    /**
     * Create a interface name reference from a u8 slice.
     *
     * Users should be careful to ensure all the following
     * requirements are met:
     *
     * dbus spec 0.26 requires:
     *  composed of 1 or more elements separated by a period ('.') character.
     *  Elements contain at least 1 character
     *  Elements must contain only the ASCII characters '[A-Z][a-z][0-9]_' and must not begin with
     *    a digit
     *  Interface names must contain at least one '.' character (and thus at least 2 elements)
     *  Interface names must not being with a '.' character
     * sd-bus additionally requires nul ('\0') termination of the interface name.
     */
    pub fn from_bytes(b: &[u8]) -> result::Result<&InterfaceName, &'static str> {
        if b.is_empty() {
            return Err("Name must have more than 0 characters");
        }

        match b[0] {
            b'.' => return Err("Name must not begin with '.'"),
            b'A'..=b'Z' | b'a'..=b'z' | b'_' => {
                // Ok
            }
            _ => return Err("Name must only begin with '[A-Z][a-z]_'"),
        }

        let mut periods = 0;
        for w in b.windows(2) {
            let prev = w[0];
            let c = w[1];
            match c {
                b'.' => {
                    if prev == b'.' {
                        return Err("Name must not have 2 '.' next to each other");
                    }

                    periods += 1;
                }
                b'A'..=b'Z' | b'a'..=b'z' | b'_' => {
                    // Ok
                }
                b'0'..=b'9' => {
                    if prev == b'.' {
                        return Err("Name element must not start with '[0-9]'");
                    }
                    // otherwise, Ok
                }
                b'\0' => {
                    if prev == b'.' && b.len() != 1 {
                        return Err("Name must not end in '.'");
                    }

                    if periods < 1 {
                        return Err("Name must have at least 2 elements");
                    }
                    return Ok(unsafe { InterfaceName::from_bytes_unchecked(b) });
                }
                _ => {
                    return Err(
                        "Invalid character in interface name, only '[A-Z][a-z][0-9]_\\.' \
                                allowed",
                    );
                }
            }
        }

        Err("Name must be terminated in a '\\0' byte (for use by sd-bus)")
    }

    /// # Safety
    ///
    ///  - `b` must be a nul terminated string
    ///  - `b` must contain a valid interface string
    #[inline]
    pub unsafe fn from_bytes_unchecked(b: &[u8]) -> &InterfaceName {
        &*(b as *const [u8] as *const InterfaceName)
    }

    /// # Safety
    ///
    ///  - `b` must have a lifetime of at least `'a`
    ///  - `b` must be a nul terminated string
    ///  - `b` must contain a valid interface string
    #[inline]
    pub unsafe fn from_ptr_unchecked<'a>(b: *const c_char) -> &'a Self {
        Self::from_bytes_unchecked(CStr::from_ptr(b).to_bytes_with_nul())
    }
}

impl Deref for InterfaceName {
    type Target = CStr;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[test]
fn t_interface() {
    InterfaceName::from_bytes(b"12\0").err().unwrap();
    InterfaceName::from_bytes(b"a\0").err().unwrap();
    InterfaceName::from_bytes(b"a.b\0").unwrap();
    InterfaceName::from_bytes(b"a.b.3\0").err().unwrap();
    InterfaceName::from_bytes(b"A.Z.xar.yfds.d3490\0").unwrap();
    InterfaceName::from_bytes(b"a.b.c\0").unwrap();
    InterfaceName::from_bytes(b"a.b.c?\0").err().unwrap();
}

#[derive(Debug)]
pub struct BusName {
    inner: CStr,
}

impl BusName {
    /**
     * Create a bus name reference from a u8 slice.
     *
     * Users should be careful to ensure all the following
     * requirements are met:
     *
     * dbus spec 0.26 requires:
     *  unique names start with a ':'. well-known names do not.
     *  composed of one or more elements separated by a period '.'
     *  all elements must be at least 1 character
     *  elements can contain only the ASCII characters '[A-Z][a-z][0-9]_-'.
     *  elements part of a unique name may begin with a digit. elements in all other bus names must
     *    not begin with a digit.
     *  must contain at least 1 '.', and thus at least 2 elements
     *  must not begin with '.'
     *  must be less than the maximum name length (255)
     *
     * sd-bus additionally requires nul ('\0') termination of the bus name.
     */
    pub fn from_bytes(b: &[u8]) -> result::Result<&Self, &'static str> {
        if b.is_empty() {
            return Err("Name must have more than 0 characters");
        }

        if b.len() > 256 {
            return Err("Must be shorter than 255 characters");
        }

        let mut is_unique = false;
        match b[0] {
            b'.' => return Err("Name must not begin with '.'"),
            b'A'..=b'Z' | b'a'..=b'z' | b'_' | b'-' => {
                // Ok
            }
            b':' => {
                is_unique = true; /* Ok */
            }
            _ => return Err("Name must only begin with '[A-Z][a-z]_'"),
        }

        let mut periods = 0;
        for w in b.windows(2) {
            let prev = w[0];
            let c = w[1];
            match c {
                b'.' => {
                    if prev == b'.' || prev == b':' {
                        return Err("Elements may not be empty");
                    }

                    periods += 1;
                }
                b'A'..=b'Z' | b'a'..=b'z' | b'_' | b'-' => {
                    // Ok
                }
                b'0'..=b'9' => {
                    if prev == b'.' && !is_unique {
                        return Err("Name element must not start with '[0-9]'");
                    }
                    // otherwise, Ok
                }
                b'\0' => {
                    if prev == b'.' && b.len() != 1 {
                        return Err("Name must not end in '.'");
                    }

                    if periods < 1 {
                        return Err("Name must have at least 2 elements");
                    }
                    return Ok(unsafe { BusName::from_bytes_unchecked(b) });
                }
                _ => {
                    return Err(
                        "Invalid character in bus name, only '[A-Z][a-z][0-9]_\\.' allowed",
                    );
                }
            }
        }

        Err("Name must be terminated in a '\\0' byte (for use by sd-bus)")
    }

    /// # Safety
    ///
    /// - `b` must be nul (`'\0'`) terminated
    /// - `b` must be a valid bus name string
    #[inline]
    pub unsafe fn from_bytes_unchecked(b: &[u8]) -> &Self {
        &*(b as *const [u8] as *const BusName)
    }

    /// # Safety
    ///
    /// - `b` must have a lifetime of at least `'a`
    /// - `b` must be nul (`'\0'`) terminated
    /// - `b` must be a valid bus name string
    #[inline]
    pub unsafe fn from_ptr_unchecked<'a>(b: *const c_char) -> &'a Self {
        Self::from_bytes_unchecked(CStr::from_ptr(b).to_bytes())
    }
}

impl Deref for BusName {
    type Target = CStr;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[test]
fn t_busname() {
    BusName::from_bytes(b"a.b\0").unwrap();
    BusName::from_bytes(b"a.b").err().unwrap();
    BusName::from_bytes(b"a\0").err().unwrap();
    BusName::from_bytes(b"a.b?\0").err().unwrap();
    BusName::from_bytes(b"a.b-c.a0\0").unwrap();
    BusName::from_bytes(b"a.b-c.0a\0").err().unwrap();
    BusName::from_bytes(b":a.b-c\0").unwrap();
    BusName::from_bytes(b":a.b-c.1\0").unwrap();
}

#[derive(Debug)]
pub struct MemberName {
    inner: CStr,
}

impl MemberName {
    /**
     * Create a member name reference from a u8 slice.
     *
     * Users should be careful to ensure all the following
     * requirements are met:
     *
     * dbus spec 0.26 requires:
     *  must only contain the ASCII characters '[A-Z][a-z][0-9]_' and may not begin with a digit
     *  must not contain the '.' character
     *  must not exceed the maximum name length (255)
     *  must be at least 1 byte in length
     *
     * sd-bus additionally requires nul ('\0') termination of the bus name.
     */
    pub fn from_bytes(b: &[u8]) -> result::Result<&Self, &'static str> {
        if b.len() < 2 {
            return Err("Name must have more than 0 characters");
        }

        if b.len() > 256 {
            return Err("Must be shorter than 255 characters");
        }

        match b[0] {
            b'A'..=b'Z' | b'a'..=b'z' | b'_' => {
                // Ok
            }
            _ => return Err("Must begin with '[A-Z][a-z]_'"),
        }

        for c in b {
            match *c {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' => {
                    // Ok
                }
                b'\0' => return Ok(unsafe { Self::from_bytes_unchecked(b) }),
                _ => {
                    return Err(
                        "Invalid character in member name, only '[A-Z][a-z][0-9]_' allowed",
                    );
                }
            }
        }

        Err("Name must be terminated in a '\\0' byte (for use by sd-bus)")
    }

    /// # Safety
    ///
    /// `b` must be a valid c-string (ie: it must be `\0` (nul) terminated).
    #[inline]
    pub unsafe fn from_bytes_unchecked(b: &[u8]) -> &Self {
        &*(b as *const [u8] as *const MemberName)
    }

    /// # Safety
    ///
    /// `b` must point to a valid c-string, with lifetime at least `'a`
    #[inline]
    pub unsafe fn from_ptr_unchecked<'a>(b: *const c_char) -> &'a Self {
        Self::from_bytes_unchecked(CStr::from_ptr(b).to_bytes())
    }
}

impl Deref for MemberName {
    type Target = CStr;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[test]
fn t_member_name() {
    MemberName::from_bytes(b"abc13\0").unwrap();
    MemberName::from_bytes(b"abc.13\0").err().unwrap();
    MemberName::from_bytes(b"1234abc\0").err().unwrap();
    MemberName::from_bytes(b"abc").err().unwrap();
    MemberName::from_bytes(b"\0").err().unwrap();
    MemberName::from_bytes(b"a\0").unwrap();
}

/*
/// Representation of a callback that may occur in the future.
///
/// XXX: when does fiddling with these cause callbacks to get de-registered. Do they ever get
/// de-registered?
struct Slot {
    raw: *mut ffi::sd_bus_slot,
}

struct SlotRef
    _inner: ffi::sd_bus_slot,
}

impl Slot {


}
*/

/*
/// These correspond to the flags passed to [`sd_bus_request_name()`]
///
/// [`sd_bus_request_name`]: https://www.freedesktop.org/software/systemd/man/sd_bus_request_name.html
#[derive(EnumFlags,Copy,Clone,Debug,PartialEq,Eq)]
#[repr(u64)]
pub enum NameFlags {
    /// After acquiring the name successfully, permit other peers to take over the name when they
    /// try to acquire it with `ReplaceExisting`.
    // XXX: add dbus meaning
    AllowReplacement = 1<<0,

    /// Take over the name if it is already acquired by another peer, and that other peer has
    /// permitted takeover by setting `AllowReplacement` when acquiring it.
    // XXX: add dbus meaning
    ReplaceExisting = 1<<1,

    /// Queue the acquisition of the name when the name is already taken.
    Queue = 1<<2,
}
*/

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    MethodCall,
    MethodReturn,
    MethodError,
    Signal,
}

impl MessageType {
    pub fn from_raw(raw: u8) -> Self {
        match raw as c_int {
            ffi::bus::SD_BUS_MESSAGE_METHOD_CALL => MessageType::MethodCall,
            ffi::bus::SD_BUS_MESSAGE_METHOD_RETURN => MessageType::MethodReturn,
            ffi::bus::SD_BUS_MESSAGE_METHOD_ERROR => MessageType::MethodError,
            ffi::bus::SD_BUS_MESSAGE_SIGNAL => MessageType::Signal,
            _ => panic!(),
        }
    }
}

/*
impl enumflags2::BitFlags<NameFlags> {
    fn as_raw(&self) -> u64 {
        let mut v = 0;
        for f in self.iter() {
            match f {
                NameFlags::AllowReplacement => { v |= ffi::SD_BUS_NAME_ALLOW_REPLACEMENT },
                NameFlags::ReplaceExisting  => { v |= ffi::SD_BUS_NAME_REPLACE_EXISTING },
                NameFlags::Queue => { v |= ffi::SD_BUS_NAME_QUEUE },
            }
        }
    }
}
*/

// TODO: consider providing a duplicate of this that promises it contains an error
// We need this more general one for writing more direct interfaces into sd-bus, but most user code
// will only encounter an error that is correctly populated by sd-bus itself.
#[repr(C)]
pub struct RawError {
    inner: ffi::bus::sd_bus_error,
}

impl RawError {
    /// # Safety
    ///
    /// `ptr` must point to a valid `sd_bus_error` which has a lifetime of at least `'a`.
    pub unsafe fn from_ptr<'a>(ptr: *const ffi::bus::sd_bus_error) -> &'a Self {
        // this is incredibly questionable: we're casting it through to a wrapper struct. It's
        // unclear if we're providing everything necessary for this to work right.
        //
        // This probably indicates we should get rid of the concrete/cached `Error` as we can't
        // make this into an `Error` without coping it.
        &*(ptr as *const _ as *const RawError)
    }
}

pub struct Error {
    raw: RawError,
    name_len: usize,
    message_len: usize,
}

impl Error {
    /// # Safety
    ///
    /// - `raw` must be populated with valid pointers, is it is if returned by another sd_bus api.
    unsafe fn from_raw(raw: RawError) -> Error {
        let n = CStr::from_ptr(raw.inner.name).to_bytes_with_nul().len();
        let m = if raw.inner.message.is_null() {
            0
        } else {
            CStr::from_ptr(raw.inner.message).to_bytes_with_nul().len()
        };

        Error {
            raw,
            name_len: n,
            message_len: m,
        }
    }

    pub fn new(name: &Utf8CStr, message: Option<&Utf8CStr>) -> Error {
        let v = RawError::with(name, message);

        Error {
            raw: v,
            name_len: name.len() + 1,
            message_len: message.map_or(0, |x| x.len() + 1),
        }
    }

    pub fn name(&self) -> &Utf8CStr {
        unsafe { Utf8CStr::from_raw_parts(self.raw.inner.name, self.name_len) }
    }

    pub fn message(&self) -> Option<&Utf8CStr> {
        let p = self.raw.inner.message;
        if p.is_null() {
            None
        } else {
            Some(unsafe { Utf8CStr::from_raw_parts(self.raw.inner.message, self.message_len) })
        }
    }

    fn as_ptr(&self) -> *const ffi::bus::sd_bus_error {
        self.raw.as_ptr()
    }

    unsafe fn move_into(self, dest: *mut ffi::bus::sd_bus_error) {
        let x = ::std::ptr::read(&self.raw.inner);
        forget(self);
        *dest = x;
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match self.message() {
            Some(m) => m.as_ref(),
            None => self.name().as_ref(),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Error")
            .field("name", &self.name())
            .field("message", &self.message())
            .field("need_free", &self.raw.inner.need_free)
            .finish()
    }
}

// TODO: make this display nicer
impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.message() {
            Some(m) => write!(fmt, "Dbus Error: {}: {}", self.name(), m),
            None => write!(fmt, "Dbus Error: {}", self.name()),
        }
    }
}

impl Default for RawError {
    #[inline]
    fn default() -> Self {
        RawError {
            inner: ffi::bus::sd_bus_error {
                name: ptr::null(),
                message: ptr::null(),
                need_free: 0,
            },
        }
    }
}

impl From<ffi::bus::sd_bus_error> for RawError {
    fn from(inner: ffi::bus::sd_bus_error) -> Self {
        Self { inner }
    }
}

impl RawError {
    #[inline]
    fn new() -> Self {
        Default::default()
    }

    fn into_result(self) -> Result<()> {
        if self.is_set() {
            Err(unsafe { Error::from_raw(self) })
        } else {
            Ok(())
        }
    }

    fn with(name: &Utf8CStr, message: Option<&Utf8CStr>) -> Self {
        let mut v: Self = Default::default();
        v.set(name, message);
        v
    }

    // XXX: if error is already set, this will not update the error
    // WARNING: using error_set causes strlen() usage even though we already have the lengths
    fn set(&mut self, name: &Utf8CStr, message: Option<&Utf8CStr>) {
        /* return value of sd_bus_error_set is calculated based on name, which we don't care about
         * */
        unsafe {
            ffi::bus::sd_bus_error_set(
                &mut self.inner,
                name.as_ptr(),
                message.map_or(ptr::null(), |x| x.as_ptr()),
            );
        }
    }

    #[inline]
    fn is_set(&self) -> bool {
        !self.inner.name.is_null()
    }

    #[inline]
    fn as_mut_ptr(&mut self) -> *mut ffi::bus::sd_bus_error {
        &mut self.inner
    }

    #[inline]
    fn as_ptr(&self) -> *const ffi::bus::sd_bus_error {
        &self.inner
    }

    // XXX: watch out! this method is doing strlen() on every single call to properly construct the
    // reference. Consider caching length info somewhere.
    #[inline]
    pub fn name(&self) -> Option<&InterfaceName> {
        if self.is_set() {
            Some(unsafe { InterfaceName::from_ptr_unchecked(self.inner.name) })
        } else {
            None
        }
    }

    // XXX: watch out! this method is doing strlen() on every single call to properly construct the
    // reference. Consider caching length info somewhere.
    #[inline]
    pub fn message(&self) -> Option<&Utf8CStr> {
        if self.is_set() {
            Some(unsafe { Utf8CStr::from_ptr_unchecked(self.inner.name) })
        } else {
            None
        }
    }

    // TODO: check if the ffi function can fail, and if so in what way
    #[allow(dead_code)]
    #[inline]
    pub fn errno(&self) -> Option<c_int> {
        if self.is_set() {
            Some(unsafe { ffi::bus::sd_bus_error_get_errno(self.as_ptr()) })
        } else {
            None
        }
    }
}

impl Drop for RawError {
    #[inline]
    fn drop(&mut self) {
        unsafe { ffi::bus::sd_bus_error_free(&mut self.inner) };
    }
}

impl Clone for RawError {
    #[inline]
    fn clone(&self) -> RawError {
        let mut e = MaybeUninit::<ffi::bus::sd_bus_error>::uninit();
        unsafe { ffi::bus::sd_bus_error_copy(e.as_mut_ptr(), &self.inner) };
        let e = unsafe { e.assume_init() };
        e.into()
    }
}

impl fmt::Debug for RawError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("RawError")
            .field("name", &self.name())
            .field("message", &self.message())
            .field("need_free", &self.inner.need_free)
            .finish()
    }
}

// TODO: make this display nicer
impl fmt::Display for RawError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("RawError")
            .field("name", &self.name())
            .field("message", &self.message())
            .field("need_free", &self.inner.need_free)
            .finish()
    }
}

#[test]
fn t_raw_error() {
    let name = Utf8CStr::from_bytes(b"name\0").unwrap();
    let message = Utf8CStr::from_bytes(b"error\0").unwrap();
    RawError::new().set(name, Some(message))
}

/* XXX: fixme: return code does have meaning! */
extern "C" fn raw_message_handler<F>(
    msg: *mut ffi::bus::sd_bus_message,
    userdata: *mut c_void,
    ret_error: *mut ffi::bus::sd_bus_error,
) -> c_int
where
    F: Fn(&mut MessageRef) -> Result<()>,
{
    let m: Box<F> = unsafe { Box::from_raw(userdata as *mut F) };
    let e = m(unsafe { MessageRef::from_ptr_mut(msg) });

    match e {
        Err(e) => {
            /* XXX: this relies on ret_error not being allocated data, otherwise we'll leak. */
            unsafe { e.move_into(ret_error) }
            /* If negative, sd_bus_reply_method_errno() is used, which should also work, but this
             * is more direct */
            0
        }
        Ok(_) => {
            /* FIXME: 0 vs positive return codes have different meaning. need to expose/chose
             * properly here */
            0
        }
    }
}

extern "C" fn raw_destroy_cb_message_handler<F>(userdata: *mut c_void)
where
    F: Fn(&mut MessageRef) -> Result<()>,
{
    let _: Box<F> = unsafe { Box::from_raw(userdata as *mut F) };
}

foreign_type! {
    pub unsafe type Bus {
        type CType = ffi::bus::sd_bus;
        fn drop = ffi::bus::sd_bus_unref;
        fn clone = ffi::bus::sd_bus_ref;
    }
}

impl Bus {
    // TODO: consider renaming all these methods so we don't have this one named `default()`, which
    // confuses things with std::default::Default::default.
    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn default() -> crate::Result<Bus> {
        let mut b = MaybeUninit::uninit();
        sd_try!(ffi::bus::sd_bus_default(b.as_mut_ptr()));
        Ok(unsafe { Bus::from_ptr(b.assume_init()) })
    }

    #[inline]
    pub fn default_user() -> crate::Result<Bus> {
        let mut b = MaybeUninit::uninit();
        sd_try!(ffi::bus::sd_bus_default_user(b.as_mut_ptr()));
        Ok(unsafe { Bus::from_ptr(b.assume_init()) })
    }

    #[inline]
    pub fn default_system() -> super::Result<Bus> {
        let mut b = MaybeUninit::uninit();
        sd_try!(ffi::bus::sd_bus_default_system(b.as_mut_ptr()));
        Ok(unsafe { Bus::from_ptr(b.assume_init()) })
    }
}

impl fmt::Debug for BusRef {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("BusRef")
            .field("unique_name", &self.unique_name())
            .field("bus_id", &self.bus_id())
            .field("scope", &self.scope())
            .field("tid", &self.tid())
            //.field("owner_creds", &self.owner_creds())
            .field("description", &self.description())
            //.field("is_server", &self.is_server())
            //.field("is_bus_client", &self.is_bus_client())
            .field("address", &self.address())
            //.field("is_trusted", &self.is_trusted())
            //.field("is_anonymous", &self.is_anonymous())
            //.field("is_monitor", &self.is_monitor())
            //.field("is_open", &self.is_open())
            //.field("is_ready", &self.is_ready())
            .field("fd", &self.fd())
            .field("events", &self.events())
            .field("n_queued_read", &self.n_queued_read())
            .field("n_queued_write", &self.n_queued_write())
            //.field("event", &self.event())
            .field("method_call_timeout", &self.method_call_timeout())
            .finish()
        // Consider:
        // current_message
        // current_handler
        // current_slot
        //
        // Also consider removing some of those included. It's a bit bulky
    }
}

impl BusRef {
    /// Returns the file descriptor used to communicate from a message bus object. This descriptor
    /// can be used with `poll(3)` or a similar function to wait for I/O events on the specified
    /// bus connection object.
    ///
    /// This corresponds to [`sd_bus_get_fd`]
    ///
    /// [`sd_bus_get_fd`]: https://www.freedesktop.org/software/systemd/man/sd_bus_get_fd.html
    #[inline]
    pub fn fd(&self) -> super::Result<c_int> {
        Ok(sd_try!(ffi::bus::sd_bus_get_fd(self.as_ptr())))
    }

    /// Returns the I/O events to wait for, suitable for passing to poll or a similar call.
    /// Returns a combination of `POLLIN`, `POLLOUT`, ... events.
    ///
    /// This corresponds to [`sd_bus_get_events`].
    ///
    /// [`sd_bus_get_events`]: https://www.freedesktop.org/software/systemd/man/sd_bus_get_events.html
    #[inline]
    pub fn events(&self) -> super::Result<c_int> {
        Ok(sd_try!(ffi::bus::sd_bus_get_events(self.as_ptr())))
    }

    /// Returns the time-out in us to pass to `poll()` or a similar call when waiting for events on
    /// the specified bus connection.
    ///
    /// This corresponds to [`sd_bus_get_timeout`].
    ///
    /// [`sd_bus_get_timeout`]: https://www.freedesktop.org/software/systemd/man/sd_bus_get_timeout.html
    #[inline]
    pub fn timeout(&self) -> super::Result<u64> {
        let mut b = MaybeUninit::uninit();
        sd_try!(ffi::bus::sd_bus_get_timeout(self.as_ptr(), b.as_mut_ptr()));
        let b = unsafe { b.assume_init() };
        Ok(b)
    }

    /// Drives the connection between the client and the message bus.
    /// Each time it is invoked a single operation is executed.
    ///
    /// Returns `None` if no operations were pending (and thus no operations were processed).
    /// Returns `Some(None)` if progress was made but no message was processed.
    /// Returns `Some(Message)` if a message was processed.
    ///
    ///
    /// This corresponds to [`sd_bus_process`].
    ///
    /// [`sd_bus_process`]: https://www.freedesktop.org/software/systemd/man/sd_bus_process.html
    #[inline]
    pub fn process(&mut self) -> super::Result<Option<Option<Message>>> {
        let mut b = MaybeUninit::uninit();
        let r = sd_try!(ffi::bus::sd_bus_process(self.as_ptr(), b.as_mut_ptr()));
        if r > 0 {
            let b = unsafe { b.assume_init() };
            if b.is_null() {
                Ok(Some(None))
            } else {
                Ok(Some(Some(unsafe { Message::from_ptr(b) })))
            }
        } else {
            Ok(None)
        }
    }

    /// This corresponds to [`sd_bus_process_priority`].
    ///
    /// [`sd_bus_process_priority`]: https://www.freedesktop.org/software/systemd/man/sd_bus_process_priority.html
    #[inline]
    pub fn process_priority(
        &mut self,
        max_priority: i64,
    ) -> super::Result<Option<Option<Message>>> {
        let mut b = MaybeUninit::uninit();
        let r = sd_try!(ffi::bus::sd_bus_process_priority(
            self.as_ptr(),
            max_priority,
            b.as_mut_ptr()
        ));
        if r > 0 {
            let b = unsafe { b.assume_init() };
            if b.is_null() {
                Ok(Some(None))
            } else {
                Ok(Some(Some(unsafe { Message::from_ptr(b) })))
            }
        } else {
            Ok(None)
        }
    }

    /// Synchronously waits for pending I/O on this `Bus` object.
    ///
    /// After each invocation of `wait()`, `process()` should be invoked to process pending I/O
    /// work.
    ///
    /// Returns `true` if any I/O was seen.
    ///
    ///
    /// This corresponds to [`sd_bus_wait`].
    ///
    /// [`sd_bus_wait`]: https://www.freedesktop.org/software/systemd/man/sd_bus_wait.html
    #[inline]
    pub fn wait(&mut self, timeout: Option<Duration>) -> super::Result<bool> {
        Ok(sd_try!(ffi::bus::sd_bus_wait(
            self.as_ptr(),
            timeout.map(usec_from_duration).unwrap_or(u64::MAX)
        )) > 0)
    }

    /// Get the unique name (address) of this connection to this `Bus`.
    ///
    ///
    ///
    /// This corresponds to [`sd_bus_get_unique_name`].
    ///
    /// [`sd_bus_get_unique_name`]: https://www.freedesktop.org/software/systemd/man/sd_bus_get_unique_name.html
    #[inline]
    pub fn unique_name(&self) -> super::Result<&BusName> {
        let mut e = MaybeUninit::uninit();
        sd_try!(ffi::bus::sd_bus_get_unique_name(
            self.as_ptr(),
            e.as_mut_ptr()
        ));
        let e = unsafe { e.assume_init() };
        Ok(unsafe { BusName::from_ptr_unchecked(e) })
    }

    pub fn scope(&self) -> super::Result<&CStr> {
        let mut ret = ptr::null();
        sd_try!(ffi::bus::sd_bus_get_scope(self.as_ptr(), &mut ret));
        Ok(unsafe { CStr::from_ptr(ret) })
    }

    pub fn tid(&self) -> super::Result<pid_t> {
        let mut ret = 0;
        sd_try!(ffi::bus::sd_bus_get_tid(self.as_ptr(), &mut ret));
        Ok(ret)
    }

    // pub fn owner_creds(&self, creds_mask: u64) -> super::Result<sd_bus_creds>

    pub fn description(&self) -> super::Result<&CStr> {
        let mut ret = ptr::null();
        sd_try!(ffi::bus::sd_bus_get_description(self.as_ptr(), &mut ret));
        Ok(unsafe { CStr::from_ptr(ret) })
    }

    pub fn address(&self) -> super::Result<&CStr> {
        let mut ret = ptr::null();
        sd_try!(ffi::bus::sd_bus_get_address(self.as_ptr(), &mut ret));
        Ok(unsafe { CStr::from_ptr(ret) })
    }

    /*
            .field("is_server", &self.is_server())
            .field("is_bus_client", &self.is_bus_client())
            .field("address", &self.address())
            .field("is_trusted", &self.is_trusted())
            .field("is_anonymous", &self.is_anonymous())
            .field("is_monitor", &self.is_monitor())
            .field("is_open", &self.is_open())
            .field("is_ready", &self.is_ready())
    */

    pub fn n_queued_write(&self) -> super::Result<u64> {
        let mut ret = Default::default();
        sd_try!(ffi::bus::sd_bus_get_n_queued_write(self.as_ptr(), &mut ret));
        Ok(ret)
    }

    pub fn n_queued_read(&self) -> super::Result<u64> {
        let mut ret = Default::default();
        sd_try!(ffi::bus::sd_bus_get_n_queued_read(self.as_ptr(), &mut ret));
        Ok(ret)
    }

    /*
    pub fn event(&self) -> super::Result<Event>
    {

    }
    */

    pub fn method_call_timeout(&self) -> super::Result<u64> {
        let mut ret = Default::default();
        sd_try!(ffi::bus::sd_bus_get_method_call_timeout(
            self.as_ptr(),
            &mut ret
        ));
        Ok(ret)
    }

    pub fn bus_id(&self) -> super::Result<super::id128::Id128> {
        let mut id: super::id128::Id128 = Default::default();
        crate::ffi_result(unsafe { ffi::bus::sd_bus_get_bus_id(self.as_ptr(), id.as_raw_mut()) })?;
        Ok(id)
    }

    ///
    /// This corresponds to [`sd_bus_message_new_signal`].
    ///
    /// [`sd_bus_message_new_signal`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_new_signal.html
    #[inline]
    pub fn new_signal(
        &mut self,
        path: &ObjectPath,
        interface: &InterfaceName,
        member: &MemberName,
    ) -> super::Result<Message> {
        let mut m = MaybeUninit::uninit();
        sd_try!(ffi::bus::sd_bus_message_new_signal(
            self.as_ptr(),
            m.as_mut_ptr(),
            path.as_ptr() as *const _,
            interface.as_ptr() as *const _,
            member.as_ptr() as *const _
        ));
        let m = unsafe { m.assume_init() };
        Ok(unsafe { Message::from_ptr(m) })
    }

    /// This corresponds to [`sd_bus_message_new_method_call`].
    ///
    /// [`sd_bus_message_new_method_call`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_new_method_call.html
    #[inline]
    pub fn new_method_call(
        &mut self,
        dest: &BusName,
        path: &ObjectPath,
        interface: &InterfaceName,
        member: &MemberName,
    ) -> super::Result<Message> {
        let mut m = MaybeUninit::uninit();
        sd_try!(ffi::bus::sd_bus_message_new_method_call(
            self.as_ptr(),
            m.as_mut_ptr(),
            dest as *const _ as *const _,
            path as *const _ as *const _,
            interface as *const _ as *const _,
            member as *const _ as *const _
        ));
        let m = unsafe { m.assume_init() };
        Ok(unsafe { Message::from_ptr(m) })
    }

    // new_method_errno

    // TODO: consider using a guard object for name handling
    /// This blocks. To get async behavior, use `request_name_async()`
    ///
    ///
    /// This corresponds to [`sd_bus_request_name`]
    ///
    /// [`sd_bus_request_name`]: https://www.freedesktop.org/software/systemd/man/sd_bus_request_name.html
    #[inline]
    pub fn request_name(&mut self, name: &BusName, flags: u64) -> super::Result<()> {
        sd_try!(ffi::bus::sd_bus_request_name(
            self.as_ptr(),
            name as *const _ as *const _,
            flags
        ));
        Ok(())
    }

    #[inline]
    pub fn request_name_async<F>(
        &mut self,
        name: &BusName,
        flags: u64,
        callback: F,
    ) -> super::Result<()>
    where
        F: Fn(&mut MessageRef) -> Result<()> + Send + Sync + 'static,
    {
        let f: extern "C" fn(
            *mut ffi::bus::sd_bus_message,
            *mut c_void,
            *mut ffi::bus::sd_bus_error,
        ) -> c_int = raw_message_handler::<F>;
        let d: extern "C" fn(*mut c_void) = raw_destroy_cb_message_handler::<F>;
        let mut slot = ptr::null_mut();
        let b = Box::into_raw(Box::new(callback));
        match unsafe {
            crate::ffi_result(ffi::bus::sd_bus_request_name_async(
                self.as_ptr(),
                &mut slot,
                name as *const _ as *const _,
                flags,
                Some(f),
                b as *mut c_void,
            ))
        } {
            Err(e) => {
                // try not to leak
                let _ = unsafe { Box::from_raw(b) };
                Err(e)
            }
            Ok(_) => {
                unsafe {
                    ffi::bus::sd_bus_slot_set_destroy_callback(slot, Some(d));
                    // we don't want to take care of this one, let the bus handle it
                    ffi::bus::sd_bus_slot_set_floating(slot, 1);
                }
                Ok(())
            }
        }
    }

    /// This blocks. To get async behavior, use `request_name` directly.
    #[inline]
    pub fn release_name(&self, name: &BusName) -> super::Result<()> {
        sd_try!(ffi::bus::sd_bus_release_name(
            self.as_ptr(),
            name as *const _ as *const _
        ));
        Ok(())
    }

    /// This corresponds to [`sd_bus_add_object`]
    ///
    /// [`sd_bus_add_object`]: https://www.freedesktop.org/software/systemd/man/sd_bus_add_object.html
    #[inline]
    pub fn add_object<F>(&self, path: &ObjectPath, callback: F) -> super::Result<()>
    where
        F: Fn(&mut MessageRef) -> Result<()> + Send + Sync + 'static,
    {
        let f: extern "C" fn(
            *mut ffi::bus::sd_bus_message,
            *mut c_void,
            *mut ffi::bus::sd_bus_error,
        ) -> c_int = raw_message_handler::<F>;
        let d: extern "C" fn(*mut c_void) = raw_destroy_cb_message_handler::<F>;
        let mut slot = ptr::null_mut();
        let b = Box::into_raw(Box::new(callback));
        match crate::ffi_result(unsafe {
            ffi::bus::sd_bus_add_object(
                self.as_ptr(),
                &mut slot,
                path as *const _ as *const _,
                Some(f),
                b as *mut c_void,
            )
        }) {
            Err(e) => {
                let _ = unsafe { Box::from_raw(b) };
                Err(e)
            }
            Ok(_) => {
                unsafe {
                    ffi::bus::sd_bus_slot_set_destroy_callback(slot, Some(d));
                    ffi::bus::sd_bus_slot_set_floating(slot, 1);
                }
                Ok(())
            }
        }
    }

    #[inline]
    pub fn add_object_manager(&self, path: &ObjectPath) -> super::Result<()> {
        sd_try!(ffi::bus::sd_bus_add_object_manager(
            self.as_ptr(),
            ptr::null_mut(),
            path as *const _ as *const _
        ));
        Ok(())
    }

    // pub fn add_object_vtable<T: Any + 'static>(&self,
    //                                           path: ObjectPath,
    //                                           interface: InterfaceName,
    //                                           vtable: Vtable<T>,
    //                                           userdata: T)
    //                                           -> super::Result<()> {
    //    let u = Box::into_raw(Box::new(userdata));
    //    sd_try!(ffi::bus::sd_bus_add_object_vtable(self.raw,
    //                                               ptr::null_mut(),
    //                                               path.as_ptr() as *const _,
    //                                               interface.as_ptr() as *const _,
    //                                               vtable.as_ptr(),
    //                                               Box::into_raw(Box::new(T))));
    //    Ok(())
    // }

    // emit_signal
    // emit_properties_changed
    // emit_object_added
    // emit_object_removed
    // emit_interfaces_added
    // emit_interfaces_removed

    // track
}

impl AsRawFd for BusRef {
    #[inline]
    fn as_raw_fd(&self) -> c_int {
        self.fd().unwrap()
    }
}

/*
extern "C" fn raw_track_handler<F: FnMut(Track) -> c_int>(
    track: *mut ffi::bus::sd_bus_track, userdata: *mut c_void) -> c_int
{
    let m : &mut F = unsafe { transmute(userdata) };
    m(Track::from_ptr(track))
}

pub struct Track {
    raw: *mut ffi::bus::sd_bus_track
}

impl Track {
    unsafe fn from_ptr(track: *mut ff::bus::sd_bus_track) {
        Track { raw: unsafe { ffi::bus::sd_bus_tracK_ref(tracK) } }
    }

    fn new<F: FnMut(Track)>(bus: &mut Bus, handler: F) -> super::Result<Track> {
    }
}
*/

/*
 * TODO: determine if the lifetime of a message is tied to the lifetime of the bus used to create
 * it
 */

foreign_type! {
    /// A message to be sent or that was received over dbus
    ///
    /// This is reference counted, cloned objects refer to the same root object.
    pub unsafe type Message {
        type CType = ffi::bus::sd_bus_message;
        fn drop = ffi::bus::sd_bus_message_unref;
        fn clone = ffi::bus::sd_bus_message_ref;
    }
}

/// An iterator over the elements of a `Message`, use this to read data out of a message.
///
/// Note: we're using a concrete type here instead of a reference to allow us to handle lifetimes
/// properly.
pub struct MessageIter<'a> {
    raw: *mut ffi::bus::sd_bus_message,
    life: PhantomData<&'a MessageRef>,
}

impl fmt::Debug for MessageRef {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Message")
            .field("type", &self.type_())
            .field("signature", &self.signature())
            .field("path", &self.path())
            .field("member", &self.member())
            .field("interface", &self.interface())
            .field("sender", &self.sender())
            .field("destination", &self.destination())
            .finish()
    }
}

impl MessageRef {
    /* FIXME: unclear that the mut handling is correct in all of this code (not just this function)
     * */
    /// This corresponds to [`sd_bus_message_get_bus`]
    ///
    /// [`sd_bus_message_get_bus`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_get_bus.html
    #[inline]
    pub fn bus(&self) -> &BusRef {
        unsafe { BusRef::from_ptr(ffi::bus::sd_bus_message_get_bus(self.as_ptr() as *mut _)) }
    }

    /// Set the message destination, the name of the bus client we want to send this message to.
    ///
    /// XXX: describe broadcast
    ///
    /// Fails if the message is sealed
    ///
    /// This corresponds to [`sd_bus_message_set_destination`]
    ///
    /// [`sd_bus_message_set_destination`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_set_destination.html
    #[inline]
    pub fn set_destination(&mut self, dest: &BusName) -> super::Result<()> {
        sd_try!(ffi::bus::sd_bus_message_set_destination(
            self.as_ptr(),
            dest as *const _ as *const _
        ));
        Ok(())
    }

    /// Set to true to allow the bus to launch an owner for the destination name.
    ///
    /// Set to false to prevent the bus from launching an owner for the destination name.
    ///
    /// Fails if the message is sealed
    ///
    /// ---
    ///
    /// This controls the NO_AUTO_START dbus header flag.
    ///
    /// The
    /// [specification](https://dbus.freedesktop.org/doc/dbus-specification.html#message-bus-starting-services)
    /// covers some details about the auto start mechanism, but not all of it is specified.
    ///
    /// This corresponds to [`sd_bus_message_set_auto_start`]
    ///
    /// [`sd_bus_message_set_auto_start`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_set_auto_start.html
    #[inline]
    pub fn set_auto_start(&mut self, yes: bool) -> super::Result<()> {
        sd_try!(ffi::bus::sd_bus_message_set_auto_start(
            self.as_ptr(),
            yes as c_int
        ));
        Ok(())
    }

    /// This corresponds to [`sd_bus_message_get_type`]
    ///
    /// [`sd_bus_message_get_type`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_get_type.html
    pub fn type_(&self) -> MessageType {
        let mut t = 0;
        crate::ffi_result(unsafe { ffi::bus::sd_bus_message_get_type(self.as_ptr(), &mut t) })
            .unwrap();

        MessageType::from_raw(t)
    }

    /// This corresponds to [`sd_bus_message_get_path`]
    ///
    /// [`sd_bus_message_get_path`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_get_path.html
    pub fn path(&self) -> Option<&CStr> {
        let p = unsafe { ffi::bus::sd_bus_message_get_path(self.as_ptr()) };
        if p.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(p) })
        }
    }

    /// This corresponds to [`sd_bus_message_get_interface`]
    ///
    /// [`sd_bus_message_get_interface`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_get_interface.html
    pub fn interface(&self) -> Option<&CStr> {
        let p = unsafe { ffi::bus::sd_bus_message_get_interface(self.as_ptr()) };
        if p.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(p) })
        }
    }

    /// This corresponds to [`sd_bus_message_get_member`]
    ///
    /// [`sd_bus_message_get_member`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_get_member.html
    pub fn member(&self) -> Option<&CStr> {
        let p = unsafe { ffi::bus::sd_bus_message_get_member(self.as_ptr()) };
        if p.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(p) })
        }
    }

    /// This corresponds to [`sd_bus_message_get_sender`]
    ///
    /// [`sd_bus_message_get_sender`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_get_sender.html
    pub fn sender(&self) -> &CStr {
        let p = unsafe { ffi::bus::sd_bus_message_get_sender(self.as_ptr()) };
        assert!(!p.is_null());

        unsafe { CStr::from_ptr(p) }
    }

    /// This corresponds to [`sd_bus_message_get_destination`]
    ///
    /// [`sd_bus_message_get_destination`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_get_destination.html
    pub fn destination(&self) -> &CStr {
        let p = unsafe { ffi::bus::sd_bus_message_get_destination(self.as_ptr()) };
        assert!(!p.is_null());

        unsafe { CStr::from_ptr(p) }
    }

    /// This corresponds to [`sd_bus_message_get_signature`]
    ///
    /// [`sd_bus_message_get_signature`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_get_signature.html
    // XXX: doesn't allow partial signatures
    pub fn signature(&self) -> &CStr {
        let p = unsafe { ffi::bus::sd_bus_message_get_signature(self.as_ptr(), 1) };
        assert!(!p.is_null());

        unsafe { CStr::from_ptr(p) }
    }

    /// This corresponds to [`sd_bus_message_is_empty`]
    ///
    /// [`sd_bus_message_is_empty`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_is_empty.html
    pub fn is_empty(&self) -> bool {
        crate::ffi_result(unsafe { ffi::bus::sd_bus_message_is_empty(self.as_ptr()) }).unwrap() != 0
    }

    /// This corresponds to [`sd_bus_message_get_error`]
    ///
    /// [`sd_bus_message_get_error`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_get_error.html
    pub fn error(&self) -> &RawError {
        unsafe { RawError::from_ptr(ffi::bus::sd_bus_message_get_error(self.as_ptr())) }
    }

    /// This corresponds to [`sd_bus_message_get_errno`]
    ///
    /// [`sd_bus_message_get_errno`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_get_errno.html
    pub fn errno(&self) -> c_int {
        unsafe { ffi::bus::sd_bus_message_get_errno(self.as_ptr()) }
    }

    /// This corresponds to [`sd_bus_message_get_monotonic_usec`]
    ///
    /// [`sd_bus_message_get_monotonic_usec`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_get_monotonic_usec.html
    pub fn monotonic_usec(&self) -> super::Result<u64> {
        let mut usec = 0;
        crate::ffi_result(unsafe {
            ffi::bus::sd_bus_message_get_monotonic_usec(self.as_ptr(), &mut usec)
        })?;

        Ok(usec)
    }

    /// This corresponds to [`sd_bus_message_get_realtime_usec`]
    ///
    /// [`sd_bus_message_get_realtime_usec`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_get_realtime_usec.html
    pub fn realtime_usec(&self) -> super::Result<u64> {
        let mut usec = 0;
        crate::ffi_result(unsafe {
            ffi::bus::sd_bus_message_get_realtime_usec(self.as_ptr(), &mut usec)
        })?;

        Ok(usec)
    }

    /// This corresponds to [`sd_bus_message_get_seqnum`]
    ///
    /// [`sd_bus_message_get_seqnum`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_get_seqnum.html
    pub fn seqnum(&self) -> super::Result<u64> {
        let mut seqnum = 0;
        crate::ffi_result(unsafe {
            ffi::bus::sd_bus_message_get_seqnum(self.as_ptr(), &mut seqnum)
        })?;

        Ok(seqnum)
    }

    // # properties
    // cookie
    // reply_cookie
    // priority
    // expect_reply
    // auto_start
    // allow_interactive_authorization

    // is_signal
    // is_method_call
    // is_method_error
    // has_signature

    /*
     * send (and it's wrappers below) keeps a reference to the Message, and really wants to own it
     * (it seals the message against further modification). Ideally we'd make it clearer in the API
     * that this is the case to prevent folks from accidentally trying to modify a message after
     * sending it
     */

    /// Send expecting a reply. Returns the reply cookie.
    ///
    /// Seals `self`.
    ///
    /// This corresponds to [`sd_bus_send`]
    ///
    /// [`sd_bus_send`]: https://www.freedesktop.org/software/systemd/man/sd_bus_send.html
    #[inline]
    pub fn send(&mut self) -> super::Result<u64> {
        // self.bus().send(self)
        let mut m = MaybeUninit::uninit();
        sd_try!(ffi::bus::sd_bus_send(
            ptr::null_mut(),
            self.as_ptr(),
            m.as_mut_ptr()
        ));
        let m = unsafe { m.assume_init() };
        Ok(m)
    }

    /// Send without expecting any reply
    /// Seals `self`.
    ///
    /// This corresponds to [`sd_bus_send`]
    ///
    /// [`sd_bus_send`]: https://www.freedesktop.org/software/systemd/man/sd_bus_send.html
    #[inline]
    pub fn send_no_reply(&mut self) -> super::Result<()> {
        // self.bus().send_no_reply(self)
        sd_try!(ffi::bus::sd_bus_send(
            ptr::null_mut(),
            self.as_ptr(),
            ptr::null_mut()
        ));
        Ok(())
    }

    /// Send this message to a destination.
    ///
    /// Internally, this is the same as `.set_destination()` + `.send()`
    /// Seals `self`.
    ///
    ///
    /// This corresponds to [`sd_bus_send_to`]
    ///
    /// [`sd_bus_send_to`]: https://www.freedesktop.org/software/systemd/man/sd_bus_send_to.html
    #[inline]
    pub fn send_to(&mut self, dest: &BusName) -> super::Result<u64> {
        // self.bus().send_to(self, dest)
        let mut c = MaybeUninit::uninit();
        sd_try!(ffi::bus::sd_bus_send_to(
            ptr::null_mut(),
            self.as_ptr(),
            dest as *const _ as *const _,
            c.as_mut_ptr()
        ));
        let c = unsafe { c.assume_init() };
        Ok(c)
    }

    /// Same as `self.send_to()`, but don't expect a reply.
    /// Seals `self`.
    ///
    ///
    /// This corresponds to [`sd_bus_send_to`]
    ///
    /// [`sd_bus_send_to`]: https://www.freedesktop.org/software/systemd/man/sd_bus_send_to.html
    #[inline]
    pub fn send_to_no_reply(&mut self, dest: &BusName) -> super::Result<()> {
        // self.bus().send_to_no_reply(self, dest)
        sd_try!(ffi::bus::sd_bus_send_to(
            ptr::null_mut(),
            self.as_ptr(),
            dest as *const _ as *const _,
            ptr::null_mut()
        ));
        Ok(())
    }

    /// Use this message to call a dbus method. Blocks until a reply is received or `usec`
    /// microseconds elapse (ie: this times out)
    ///
    /// XXX: document blocking forever
    /// Seals `self`.
    ///
    ///
    /// This corresponds to [`sd_bus_call`]
    ///
    /// [`sd_bus_call`]: https://www.freedesktop.org/software/systemd/man/sd_bus_call.html
    #[inline]
    pub fn call(&mut self, usec: u64) -> Result<Message> {
        let mut r = MaybeUninit::uninit();
        let mut e = RawError::new();
        unsafe {
            ffi::bus::sd_bus_call(
                ptr::null_mut(),
                self.as_ptr(),
                usec,
                e.as_mut_ptr(),
                r.as_mut_ptr(),
            );
        }
        e.into_result()
            .map(|_| unsafe { Message::from_ptr(r.assume_init()) })
    }

    // XXX: we may need to move this, unclear we have the right lifetime here (we're being too
    // strict)
    //
    /// Use this message to call a dbus method. Returns immediately and will call the callback when
    /// a reply is received.
    ///
    /// XXX: document how timeout affects this
    /// Seals `self`.
    ///
    /// This corresponds to [`sd_bus_call_async`]
    ///
    /// [`sd_bus_call_async`]: https://www.freedesktop.org/software/systemd/man/sd_bus_call_async.html
    #[inline]
    pub fn call_async<F>(&mut self, callback: F, usec: u64) -> super::Result<()>
    where
        F: Fn(&mut MessageRef) -> Result<()> + 'static + Sync + Send,
    {
        let f: extern "C" fn(
            *mut ffi::bus::sd_bus_message,
            *mut c_void,
            *mut ffi::bus::sd_bus_error,
        ) -> c_int = raw_message_handler::<F>;
        let d: extern "C" fn(*mut c_void) = raw_destroy_cb_message_handler::<F>;
        let b = Box::into_raw(Box::new(callback));
        let mut slot = ptr::null_mut();
        match crate::ffi_result(unsafe {
            ffi::bus::sd_bus_call_async(
                ptr::null_mut(),
                &mut slot,
                self.as_ptr(),
                Some(f),
                b as *mut c_void,
                usec,
            )
        }) {
            Err(e) => {
                // try not to leak
                let _ = unsafe { Box::from_raw(b) };
                Err(e)
            }
            Ok(_) => {
                unsafe {
                    ffi::bus::sd_bus_slot_set_destroy_callback(slot, Some(d));
                    // we don't want to take care of this one, let the bus handle it
                    ffi::bus::sd_bus_slot_set_floating(slot, 1);
                }
                Ok(())
            }
        }
    }

    /// This corresponds to [`sd_bus_message_new_method_error`]
    ///
    /// [`sd_bus_message_new_method_error`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_new_method_error.html
    #[inline]
    pub fn new_method_error(&mut self, error: &Error) -> crate::Result<Message> {
        let mut m = MaybeUninit::uninit();
        sd_try!(ffi::bus::sd_bus_message_new_method_error(
            self.as_ptr(),
            m.as_mut_ptr(),
            error.as_ptr()
        ));
        Ok(unsafe { Message::from_ptr(m.assume_init()) })
    }

    /// This corresponds to [`sd_bus_message_new_method_return`]
    ///
    /// [`sd_bus_message_new_method_return`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_new_method_return.html
    #[inline]
    pub fn new_method_return(&mut self) -> crate::Result<Message> {
        let mut m = MaybeUninit::uninit();
        sd_try!(ffi::bus::sd_bus_message_new_method_return(
            self.as_ptr(),
            m.as_mut_ptr()
        ));
        Ok(unsafe { Message::from_ptr(m.assume_init()) })
    }

    /// Raw access to append data to this message
    /// Will fail if the message is sealed
    ///
    /// This corresponds to [`sd_bus_message_append_basic`]
    ///
    /// # Safety
    ///
    /// The pointer `v` must point to valid data corresponding to the type indicated by `dbus_type`.
    ///
    /// [`sd_bus_message_append_basic`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_append_basic.html
    // XXX: unclear if this should operate directly on the message or be split out to the iterator
    // mechanism
    #[inline]
    pub unsafe fn append_basic_raw(
        &mut self,
        dbus_type: u8,
        v: *const c_void,
    ) -> crate::Result<()> {
        crate::ffi_result(ffi::bus::sd_bus_message_append_basic(
            self.as_ptr(),
            dbus_type as c_char,
            v,
        ))?;
        Ok(())
    }

    /// Append a value to the message
    #[inline]
    pub fn append<V: types::ToSdBusMessage>(&mut self, v: V) -> crate::Result<()> {
        v.to_message(self)
    }

    /// Get an iterator over the message. This iterator really exists with in the `Message` itself,
    /// so we can only hand out one at a time.
    ///
    /// Ideally, handing this iterator out wouldn't prevent the use of other non-iterator
    /// accessors, but right now it does (unless you bypass `borrowck` using `unsafe{}`)
    ///
    /// Requires that message is sealed.
    #[inline]
    pub fn iter(&mut self) -> crate::Result<MessageIter<'_>> {
        /* probe the `Message` to check if we can iterate on it */
        sd_try!(ffi::bus::sd_bus_message_peek_type(
            self.as_ptr(),
            ptr::null_mut(),
            ptr::null_mut()
        ));
        Ok(MessageIter {
            raw: self.as_ptr(),
            life: PhantomData,
        })
    }
}

impl<'a> MessageIter<'a> {
    #[inline]
    fn as_mut_ptr(&mut self) -> *mut ffi::bus::sd_bus_message {
        self.raw
    }

    /*
     * XXX: 'T' may reference the parent `Message`, and should be tied to the lifetime of the
     * `MessageIter` (to ensure they don't change out from underneath us) but shouldn't be tied to
     * the lifetime of the &mut self of this call
     */
    /// Read an element from the message and advance the internal cursor
    /// References returned by this function are valid until the iterator itself is dropped (just
    /// to guarantee they don't change).
    ///
    /// XXX: really, they are valid until the message is un-sealed: reading from the message can
    /// only occur while the message is sealed. Unclear if we can track lifetimes against message
    /// sealing.
    ///
    /// This corresponds to [`sd_bus_message_read_basic`]
    ///
    /// # Safety
    ///
    /// - The type `*mut R` must match the pointer expected by `sd_bus_message_read_basic()`
    ///   when it is given `dbus_type`.
    ///
    /// [`sd_bus_message_read_basic`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_read_basic.html
    #[inline]
    pub unsafe fn read_basic_raw<R, T, F: FnOnce(R) -> T>(
        &mut self,
        dbus_type: u8,
        cons: F,
    ) -> crate::Result<Option<T>>
    where
        T: 'a,
    {
        let mut v = MaybeUninit::<R>::uninit();
        match crate::ffi_result(ffi::bus::sd_bus_message_read_basic(
            self.as_mut_ptr(),
            dbus_type as c_char,
            v.as_mut_ptr() as *mut _,
        )) {
            Ok(1) => Ok(Some(cons(v.assume_init()))),
            Ok(_) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// This needs to be `&mut` as the `&str` will be invalid after either of:
    ///  - self is dropped
    ///  - sd_bus_message_peek_type is called a second time
    ///
    /// Using &mut allows us to prevent #2.
    ///
    /// FIXME/WARNING: Message might have been cloned, in which case we can't rely on the lifetime of
    /// &str! As `Message` isn't `Send` or `Sync`, we can guarantee we're not racing with someone
    /// else to free it though. Probably need to allocate space for it here rather than return a
    /// ref.
    ///
    ///
    /// This corresponds to [`sd_bus_message_peek_type`]
    ///
    /// [`sd_bus_message_peek_type`]: https://www.freedesktop.org/software/systemd/man/sd_bus_message_peek_type.html
    // &str lasts until next call of sd_bus_message_peek_type
    // XXX: confirm that lifetimes here match that!
    #[inline]
    pub fn peek_type(&mut self) -> crate::Result<(c_char, &str)> {
        let mut t = MaybeUninit::<c_char>::uninit();
        let mut cont = MaybeUninit::<*const c_char>::uninit();
        crate::ffi_result(unsafe {
            ffi::bus::sd_bus_message_peek_type(self.as_mut_ptr(), t.as_mut_ptr(), cont.as_mut_ptr())
        })?;

        let cont = unsafe { cont.assume_init() };
        let s = if cont.is_null() {
            /* XXX: we may need to adjust here and return an option, but it isn't yet clear if
             * there will be confusion between NULL and "" here */
            ""
        } else {
            unsafe { str::from_utf8_unchecked(CStr::from_ptr(cont).to_bytes()) }
        };
        let t = unsafe { t.assume_init() };
        Ok((t, s))
    }

    // XXX: handle containers
    // FIXME: consider renaming
    #[allow(clippy::should_implement_trait)]
    pub fn next<V: types::FromSdBusMessage<'a>>(&'a mut self) -> crate::Result<Option<V>> {
        V::from_message(self)
    }
}

/*
struct Vtable;
struct VtableBuilder<T> {
    Vec<ffi::bus::sd_bus_vtable>,
}

type PropertyGet<T> = fn(Bus, ObjectPath, InterfaceName, MessageRef, &mut T, &mut Error) -> c_int;
type PropertySet<T> = fn(Bus, ObjectPath, InterfaceName, MessageRef, &mut T, &mut Error) -> c_int;


impl VtableBuilder {
    fn method(mut self, member: &str, signature: &str, result: &str, handler: MessageHandler) {
        /* verify */
        /* track */
    }

    fn property(mut self, member: &str, signature: &str, get: PropertyGet) {
    }

    fn property_writable(mut self, member: &str, signature: &str, get: PropertyGet, set: PropertySet) {
    }

    fn signal(mut self, member: &str, signature: &str) {
    }

    fn create(mut self) -> Vtable {
    }
}
*/
