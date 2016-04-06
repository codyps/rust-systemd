use ffi;
use ffi::{c_int,c_char,c_void};
use std::ffi::CStr;
use std::os::unix::io::AsRawFd;
use std::mem::{uninitialized, transmute, forget};
use std::ptr;
use std::ops::Deref;

/**
 * A wrapper which promises it always holds a valid dbus object path
 */
#[derive(Debug,Clone,Copy)]
pub struct ObjectPath<'a> {
    inner: &'a [u8],
}

impl<'a> ObjectPath<'a> {
    /**
     * Create a path reference from a u8 slice.
     *
     * Users should be careful to ensure all the following
     * requirements are met:
     *
     * dbus spec 0.26 requires:
     *  path must begin with ASCII '/' and consist of elements separated by slash characters
     *  each element must only contain the ASCII characters '[A-Z][a-z][0-9]_'
     *  No element may be the empty string
     *  Multiple '/' characters may not occur in sequence
     *  A trailing '/' character is not allowed unless the path is the root path
     * sd-bus additionally requires nul ('\0') termination of paths.
     */
    pub fn from_bytes(b: &[u8]) -> Result<ObjectPath, &'static str> {


        if b.len() < 1 {
            return Err("Path must have at least 1 character ('/')");
        }

        if b[0] != b'/' as u8 {
            return Err("Path must begin with '/'");
        }

        for w in b.windows(2) {
            let prev = w[0];
            let c = w[1];

            match c {
                b'/' => { if prev == b'/' {
                    return Err("Path must not have 2 '/' next to each other");
                }},
                b'A'...b'Z'|b'a'...b'z'|b'0'...b'9'|b'_' => { /* Ok */ }
                b'\0' => {
                    if prev == b'/' && b.len() != 2 {
                        return Err("Path must not end in '/' unless it is the root path");
                    }

                    return Ok(unsafe{ObjectPath::from_bytes_unchecked(b)})
                },
                _ => {
                    return Err("Invalid character in path, only '[A-Z][a-z][0-9]_/' allowed");
                },
            }
        }

        return Err("Path must be terminated in a '\\0' byte (for use by sd-bus)");
    }

    pub unsafe fn from_bytes_unchecked(b: &[u8]) -> ObjectPath {
        ObjectPath { inner: b }
    }

    pub unsafe fn from_ptr_unchecked<'b>(b: *const c_char) -> ObjectPath<'b> {
        ObjectPath { inner: CStr::from_ptr(b).to_bytes() }
    }
}

impl<'a> Deref for ObjectPath<'a> {
    type Target = [u8];
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
    ObjectPath::from_bytes(b"/hello/goodbye/013/4?/HA\0").err().unwrap();
}

/**
 * A wrapper which promises it always holds a validated dbus interface name
 */
#[derive(Debug,Clone,Copy)]
pub struct InterfaceName<'a> {
    inner: &'a [u8]
}

impl<'a> InterfaceName<'a> {
    /**
     * Create a interface name reference from a u8 slice.
     *
     * Users should be careful to ensure all the following
     * requirements are met:
     *
     * dbus spec 0.26 requires:
     *  composed of 1 or more elements seperated by a period ('.') character.
     *  Elements contain at least 1 character
     *  Elements must contain only the ASCII characters '[A-Z][a-z][0-9]_' and must not begin with
     *    a digit
     *  Interface names must contain at least one '.' character (and thus at least 2 elements)
     *  Interface names must not being with a '.' character
     * sd-bus additionally requires nul ('\0') termination of the interface name.
     */
    pub fn from_bytes(b: &[u8]) -> Result<InterfaceName, &'static str> {

        if b.len() < 1 {
            return Err("Name must have more than 0 characters");
        }

        match b[0] {
            b'.' => return Err("Name must not begin with '.'"),
            b'A'...b'Z'|b'a'...b'z'|b'_' => { /* Ok */ },
            _ => return Err("Name must only begin with '[A-Z][a-z]_'")
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
                },
                b'A'...b'Z'|b'a'...b'z'|b'_' => { /* Ok */ }
                b'0'...b'9' => {
                    if prev == b'.' {
                        return Err("Name element must not start with '[0-9]'");
                    }
                    /* otherwise, Ok */
                }
                b'\0' => {
                    if prev == b'.' && b.len() != 1 {
                        return Err("Name must not end in '.'");
                    }

                    if periods < 1 {
                        return Err("Name must have at least 2 elements");
                    }
                    return Ok(InterfaceName { inner: b })
                },
                _ => {
                    return Err("Invalid character in interface name, only '[A-Z][a-z][0-9]_\\.' allowed");
                },
            }
        }

        return Err("Name must be terminated in a '\\0' byte (for use by sd-bus)");
    }

    pub unsafe fn from_bytes_unchecked(b: &[u8]) -> InterfaceName {
        InterfaceName { inner: b }
    }

    pub unsafe fn from_ptr_unchecked(b: *const c_char) -> Self {
        InterfaceName { inner: CStr::from_ptr(b).to_bytes() }
    }
}

impl<'a> Deref for InterfaceName<'a> {
    type Target = [u8];
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

#[derive(Debug,Clone,Copy)]
pub struct BusName<'a> {
    inner: &'a [u8],
}

impl<'a> BusName<'a> {
    /**
     * Create a bus name reference from a u8 slice.
     *
     * Users should be careful to ensure all the following
     * requirements are met:
     *
     * dbus spec 0.26 requires:
     *  unique names start with a ':'. well-known names do not.
     *  composed of one or more elemenets seperated by a period '.'
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
    pub fn from_bytes(b: &[u8]) -> Result<BusName, &'static str> {

        if b.len() < 1 {
            return Err("Name must have more than 0 characters");
        }

        if b.len() > 256 {
            return Err("Must be shorter than 255 characters");
        }

        let mut is_unique = false;
        match b[0] {
            b'.' => return Err("Name must not begin with '.'"),
            b'A'...b'Z'|b'a'...b'z'|b'_'|b'-' => { /* Ok */ },
            b':' => { is_unique = true; /* Ok */ },
            _ => return Err("Name must only begin with '[A-Z][a-z]_'")
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
                },
                b'A'...b'Z'|b'a'...b'z'|b'_'|b'-' => { /* Ok */ }
                b'0'...b'9' => {
                    if prev == b'.' && !is_unique {
                        return Err("Name element must not start with '[0-9]'");
                    }
                    /* otherwise, Ok */
                }
                b'\0' => {
                    if prev == b'.' && b.len() != 1 {
                        return Err("Name must not end in '.'");
                    }

                    if periods < 1 {
                        return Err("Name must have at least 2 elements");
                    }
                    return Ok(unsafe{BusName::from_bytes_unchecked(b)})
                },
                _ => {
                    return Err("Invalid character in bus name, only '[A-Z][a-z][0-9]_\\.' allowed");
                },
            }
        }

        return Err("Name must be terminated in a '\\0' byte (for use by sd-bus)");
    }

    pub unsafe fn from_bytes_unchecked(b: &[u8]) -> BusName {
        BusName { inner: b }
    }

    pub unsafe fn from_ptr_unchecked(b: *const c_char) -> Self {
        BusName { inner: CStr::from_ptr(b).to_bytes() }
    }
}

impl<'a> Deref for BusName<'a> {
    type Target = [u8];
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

#[derive(Debug,Clone,Copy)]
pub struct MemberName<'a> {
    inner: &'a [u8],
}

impl<'a> MemberName<'a> {
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
    pub fn from_bytes(b: &[u8]) -> Result<MemberName, &'static str> {

        if b.len() < 2 {
            return Err("Name must have more than 0 characters");
        }

        if b.len() > 256 {
            return Err("Must be shorter than 255 characters");
        }

        match b[0] {
            b'A'...b'Z'|b'a'...b'z'|b'_' => { /* Ok */ },
            _ => return Err("Must begin with '[A-Z][a-z]_'")
        }

        for c in b {
            match *c {
                b'A'...b'Z'|b'a'...b'z'|b'0'...b'9'|b'_' => { /* Ok */ }
                b'\0' => {
                    return Ok(unsafe{MemberName::from_bytes_unchecked(b)})
                },
                _ => {
                    return Err("Invalid character in member name, only '[A-Z][a-z][0-9]_' allowed");
                },
            }
        }

        return Err("Name must be terminated in a '\\0' byte (for use by sd-bus)");
    }

    pub unsafe fn from_bytes_unchecked(b: &[u8]) -> MemberName {
        MemberName { inner: b }
    }

    pub unsafe fn from_ptr_unchecked(b: *const c_char) -> Self {
        MemberName { inner: CStr::from_ptr(b).to_bytes() }
    }
}

impl<'a> Deref for MemberName<'a> {
    type Target = [u8];
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

pub struct Error {
    inner: ffi::bus::sd_bus_error,
}

impl Error {
    unsafe fn from_ptr<'a>(p: *mut ffi::bus::sd_bus_error) -> &'a mut Error
    {
        transmute(p)
    }
}

impl Drop for Error {
    fn drop(&mut self) {
        unsafe { ffi::bus::sd_bus_error_free(&mut self.inner) };
    }
}

impl Clone for Error {
    fn clone(&self) -> Error {
        let mut e = unsafe { Error { inner: uninitialized() } };
        unsafe { ffi::bus::sd_bus_error_copy(&mut e.inner, &self.inner) };
        e
    }
}

//type MessageHandler<T> = fn(Message, &mut T, &mut Error) -> c_int;
//type MessageHandler = FnMut(Message, Error) -> c_int;

extern "C" fn raw_message_handler<F: FnMut(Message, &mut Error) -> c_int>(
    msg: *mut ffi::bus::sd_bus_message,
    userdata: *mut c_void,
    ret_error: *mut ffi::bus::sd_bus_error) -> c_int
{
    let m : &mut F = unsafe { transmute(userdata) };
    unsafe { m(Message::from_ptr(msg), Error::from_ptr(ret_error)) }
}

pub struct Bus {
    raw: *mut ffi::bus::sd_bus,
}

impl Bus {
    /*
    unsafe fn take_ptr(r: *mut ffi::bus::sd_bus) -> Bus {
        Bus { raw: r }
    }

    unsafe fn from_ptr(r: *mut ffi::bus::sd_bus) -> Bus {
        unsafe { ffi::bus::sd_bus_ref(r) };
        Bus { raw: r }
    }
    */

    fn as_ptr(&mut self) -> *mut ffi::bus::sd_bus {
        self.raw
    }

    pub fn default() -> super::Result<Bus> {
        Ok(Bus { raw: unsafe {
            let mut b = uninitialized();
            sd_try!(ffi::bus::sd_bus_default(&mut b));
            b
        } } )
    }

    pub fn default_user() -> super::Result<Bus> {
        Ok(Bus { raw: unsafe {
            let mut b = uninitialized();
            sd_try!(ffi::bus::sd_bus_default_user(&mut b));
            b
        } } )
    }

    pub fn default_system() -> super::Result<Bus> {
        Ok(Bus { raw: unsafe {
            let mut b = uninitialized();
            sd_try!(ffi::bus::sd_bus_default_system(&mut b));
            b
        } } )
    }

    pub fn events(&self) -> super::Result<c_int> {
        Ok(sd_try!(ffi::bus::sd_bus_get_events(self.raw)))
    }

    pub fn timeout(&self) -> super::Result<u64> {
        Ok(unsafe {
            let mut b = uninitialized();
            sd_try!(ffi::bus::sd_bus_get_timeout(self.raw, &mut b));
            b
        })
    }

    pub fn fd(&self) -> super::Result<c_int> {
        Ok(sd_try!(ffi::bus::sd_bus_get_fd(self.raw)))
    }

    pub fn unique_name(&self) -> super::Result<BusName> {
        let mut e = unsafe { uninitialized() };
        sd_try!(ffi::bus::sd_bus_get_unique_name(self.raw, &mut e));
        Ok(unsafe { BusName::from_ptr_unchecked(e) })
    }

    /* TODO: consider using a guard object for name handling */
    pub fn request_name(&self, name: BusName, flags: u64) -> super::Result<()> {
        sd_try!(ffi::bus::sd_bus_request_name(self.raw,
                    &*name as *const _ as *const _, flags));
        Ok(())
    }

    pub fn release_name(&self, name: BusName) -> super::Result<()> {
        sd_try!(ffi::bus::sd_bus_release_name(self.raw,
                    &*name as *const _ as *const _));
        Ok(())
    }

    /* XXX: alternates for (userdata: T):
     *  - userdata: T, and automatically box as needed. Allows a useful external control.
     *  - userdata: Box<T>, allows users to supply a box directly if they already have one
     *  - userdata: &mut T, allows users to manage lifetime of passed in values direcly
     *  - userdata: SizeMatches<*const _>, allows users to use things without a pointer indirection
     *    (such as integer types). Not clear this is possible in rust today (1.9).
     *  - cb: &FnMut
     *  - cb: &CustomTrait
     */
    pub fn add_object<F: FnMut(Message, &mut Error)->c_int>(&self, path: ObjectPath, cb: &mut F) -> super::Result<()>
    {
        let f: extern "C" fn(*mut ffi::bus::sd_bus_message, *mut c_void, *mut ffi::bus::sd_bus_error) -> c_int =
            raw_message_handler::<F>;
        sd_try!(ffi::bus::sd_bus_add_object(self.raw, ptr::null_mut(), &*path as *const _ as *const _, Some(f), cb as *mut _ as *mut _));
        Ok(())
    }

    pub fn send(&self, mut message: Message) -> super::Result<u64> {
        Ok(unsafe {
            let mut c = uninitialized();
            sd_try!(ffi::bus::sd_bus_send(self.raw, message.as_mut_ptr(), &mut c));
            c
        })
    }

    pub fn send_to(&self, mut message: Message, dest: BusName) -> super::Result<u64> {
        Ok(unsafe {
            let mut c = uninitialized();
            sd_try!(ffi::bus::sd_bus_send_to(self.raw, message.as_mut_ptr(), &*dest as *const _ as *const _, &mut c));
            c
        })
    }

    pub fn add_object_manager(&self, path: ObjectPath) -> super::Result<()>
    {
        sd_try!(ffi::bus::sd_bus_add_object_manager(self.raw, ptr::null_mut(), &*path as *const _ as *const _));
        Ok(())
    }

    /*
    pub fn add_object_vtable<T: Any + 'static>(&self, path: ObjectPath, interface: InterfaceName, vtable: Vtable<T>, userdata: T) -> super::Result<()>
    {
        let u = Box::into_raw(Box::new(userdata));
        sd_try!(ffi::bus::sd_bus_add_object_vtable(self.raw, ptr::null_mut(),
                path.as_ptr() as *const _, interface.as_ptr() as *const _,
                vtable.as_ptr(), Box::into_raw(Box::new(T))
            )
        );
        Ok(())
    }
    */
}

impl AsRawFd for Bus {
    fn as_raw_fd(&self) -> c_int {
        self.fd().unwrap()
    }
}

impl Drop for Bus {
    fn drop(&mut self) {
        unsafe { ffi::bus::sd_bus_unref(self.raw) };
    }
}

impl Clone for Bus {
    fn clone(&self) -> Bus {
        Bus { raw: unsafe { ffi::bus::sd_bus_ref(self.raw) } }
    }
}

/*
 * TODO: determine if the lifetime of a message is tied to the lifetime of the bus used to create
 * it
 */
pub struct Message {
    raw: *mut ffi::bus::sd_bus_message
}

impl Message {
    unsafe fn take_ptr(p: *mut ffi::bus::sd_bus_message) -> Message
    {
        Message { raw: p }
    }

    unsafe fn from_ptr(p: *mut ffi::bus::sd_bus_message) -> Message
    {
        Message { raw: ffi::bus::sd_bus_message_ref(p) }
    }

    pub fn new_signal(bus: &mut Bus, path: ObjectPath, interface: InterfaceName, member: MemberName) -> super::Result<Message> {
        unsafe {
            let mut m = uninitialized();
            sd_try!(ffi::bus::sd_bus_message_new_signal(bus.as_ptr(), &mut m,
                path.as_ptr() as *const _, interface.as_ptr() as *const _,
                member.as_ptr() as *const _));
            Ok(Message::take_ptr(m))
        }
    }

    pub fn set_destination(&self, dest: BusName) -> super::Result<()>
    {
        sd_try!(ffi::bus::sd_bus_message_set_destination(self.raw, &*dest as *const _ as *const _));
        Ok(())
    }

    fn as_ptr(&self) -> *const ffi::bus::sd_bus_message {
        self.raw
    }

    fn as_mut_ptr(&mut self) -> *mut ffi::bus::sd_bus_message {
        self.raw
    }

    fn into_ptr(self) -> *mut ffi::bus::sd_bus_message {
        let r = self.raw;
        forget(self);
        r
    }
}

impl Drop for Message {
    fn drop(&mut self) {
        unsafe { ffi::bus::sd_bus_message_unref(self.raw) };
    }
}

impl Clone for Message {
    fn clone(&self) -> Message {
        Message { raw: unsafe { ffi::bus::sd_bus_message_ref(self.raw) } }
    }
}


/*
struct BusRef<'a> {
    life: PhantomData<&'a ()>,
    raw: *mut ffi::bus::sd_bus,
}

struct MessageRef<'a> {
    life: PhantomData<&'a ()>,
    raw: *mut ffi::bus::sd_bus_message,
}

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
