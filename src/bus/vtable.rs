use super::{BusRef, InterfaceName, MemberName, MessageIter, MessageRef, ObjectPath};
use crate::CStrArgument;
use ffi::bus::vtable::sd_bus_vtable;
use ffi::{bus as raw, c_char, c_int, c_void};
use foreign_types::ForeignTypeRef;
use std::ffi::{CStr, CString};
use std::io::{Error, ErrorKind};
use std::marker::PhantomData;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

/// Flags for vtable entries. Combine flags with `|`. The default is no flags,
/// matching libsystemd. Flags that change userdata offsets are not exposed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VtableFlags(u64);

impl VtableFlags {
    pub const DEPRECATED: Self = Self(1 << 0);
    pub const HIDDEN: Self = Self(1 << 1);
    /// Allow callers without the default privileged-method capability.
    pub const UNPRIVILEGED: Self = Self(1 << 2);
    pub const METHOD_NO_REPLY: Self = Self(1 << 3);
    pub const PROPERTY_CONST: Self = Self(1 << 4);
    /// Advertise change notifications; the application must emit the signals.
    pub const PROPERTY_EMITS_CHANGE: Self = Self(1 << 5);
    pub const PROPERTY_EMITS_INVALIDATION: Self = Self(1 << 6);
    /// Exclude this property from GetAll; clients must request it with Get.
    pub const PROPERTY_EXPLICIT: Self = Self(1 << 7);

    pub const fn empty() -> Self {
        Self(0)
    }
}

impl std::ops::BitOr for VtableFlags {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

type Handler = dyn Fn(&mut MessageRef) -> super::Result<()> + Send + Sync;
type Setter = dyn Fn(&mut MessageIter<'_>) -> super::Result<()> + Send + Sync;

enum Callbacks {
    Method(Box<Handler>),
    Property {
        get: Box<Handler>,
        set: Option<Box<Setter>>,
    },
    Signal,
}

struct Entry {
    member: CString,
    signature: CString,
    result: CString,
    flags: VtableFlags,
    callbacks: Callbacks,
}

impl Entry {
    fn raw(&self) -> sd_bus_vtable {
        match &self.callbacks {
            Callbacks::Method(_) => sd_bus_vtable::method(
                self.member.as_ptr(),
                self.signature.as_ptr(),
                self.result.as_ptr(),
                Some(method),
                0,
                self.flags.0,
            ),
            Callbacks::Property { set, .. } => sd_bus_vtable::property(
                self.member.as_ptr(),
                self.signature.as_ptr(),
                Some(get_property),
                set.as_ref()
                    .map(|_| set_property as unsafe extern "C" fn(_, _, _, _, _, _, _) -> _),
                0,
                set.is_some(),
                self.flags.0,
            ),
            Callbacks::Signal => {
                sd_bus_vtable::signal(self.member.as_ptr(), self.signature.as_ptr(), self.flags.0)
            }
        }
    }
}

/// An owned interface definition for [`BusRef::add_object_vtable`].
///
/// Member names and signatures accept Rust strings, C strings, and `Cow`.
/// Owned C strings transfer their buffers; borrowed inputs are copied when
/// owned storage is needed for registration. Names are checked
/// while building; libsystemd checks signatures and flag combinations when
/// registering. Callbacks and captured state must be `'static`; share mutable
/// state using interior mutability. The bus owns the registered definition.
///
/// Method callbacks must send a reply, arrange a deferred reply, or declare
/// `METHOD_NO_REPLY`. Returning `Ok(())` marks the request handled. Property
/// getters append the bare value to the supplied reply; setters read the bare
/// value from the supplied iterator, which is already inside the variant.
/// Errors become D-Bus error replies. Callback panics become an `EIO` failure.
///
/// This uses the v221 layout, which current libsystemd still accepts. Methods,
/// properties and signals are introspected, with unnamed arguments. Argument
/// names from the extended v242 layout are not currently supported.
///
/// ```
/// use std::sync::{Arc, atomic::{AtomicU32, Ordering}};
/// use systemd::bus::{BusRef, Vtable, VtableFlags};
///
/// fn register(bus: &BusRef) -> systemd::Result<()> {
///     let count = Arc::new(AtomicU32::new(0));
///     let get = count.clone();
///     let table = Vtable::new()
///         .method("Ping", "", "", VtableFlags::UNPRIVILEGED, |request| {
///             request.new_method_return()?.send_no_reply()?;
///             Ok(())
///         })?
///         .writable_property("Count", "u", VtableFlags::UNPRIVILEGED,
///             move |reply| { reply.append(get.load(Ordering::Relaxed))?; Ok(()) },
///             move |value| {
///                 let next = value.next::<u32>()?.ok_or_else(||
///                     std::io::Error::from(std::io::ErrorKind::UnexpectedEof))?;
///                 count.store(next, Ordering::Relaxed);
///                 Ok(())
///             })?;
///     bus.add_object_vtable("/org/example/Counter", "org.example.Counter", table)
/// }
/// ```
///
/// A callback cannot borrow state that may be dropped before its registration:
///
/// ```compile_fail
/// use systemd::bus::{Vtable, VtableFlags};
/// let count = std::sync::atomic::AtomicU32::new(0);
/// let table = Vtable::new().property("Count", "u", VtableFlags::empty(), |reply| {
///     reply.append(count.load(std::sync::atomic::Ordering::Relaxed))?;
///     Ok(())
/// });
/// ```
#[derive(Default)]
pub struct Vtable {
    entries: Vec<Entry>,
}

impl Vtable {
    /// Create an empty interface definition.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declare a method. `signature` and `result` describe sequences of input
    /// and output arguments; an empty string means no arguments.
    pub fn method<'a, F>(
        self,
        member: impl CStrArgument<'a>,
        signature: impl CStrArgument<'a>,
        result: impl CStrArgument<'a>,
        flags: VtableFlags,
        handler: F,
    ) -> crate::Result<Self>
    where
        F: Fn(&mut MessageRef) -> super::Result<()> + Send + Sync + 'static,
    {
        self.add(
            member,
            signature,
            result,
            flags,
            Callbacks::Method(Box::new(handler)),
        )
    }

    /// Declare a signal for introspection. Emit it using [`BusRef::new_signal`].
    pub fn signal<'a>(
        self,
        member: impl CStrArgument<'a>,
        signature: impl CStrArgument<'a>,
        flags: VtableFlags,
    ) -> crate::Result<Self> {
        self.add(member, signature, "", flags, Callbacks::Signal)
    }

    /// Declare a read-only property with one complete value type as its signature.
    pub fn property<'a, F>(
        self,
        member: impl CStrArgument<'a>,
        signature: impl CStrArgument<'a>,
        flags: VtableFlags,
        get: F,
    ) -> crate::Result<Self>
    where
        F: Fn(&mut MessageRef) -> super::Result<()> + Send + Sync + 'static,
    {
        self.add(
            member,
            signature,
            "",
            flags,
            Callbacks::Property {
                get: Box::new(get),
                set: None,
            },
        )
    }

    /// Declare a readable and writable property. The setter's iterator starts
    /// at the bare value inside the incoming variant, without rewinding.
    pub fn writable_property<'a, G, S>(
        self,
        member: impl CStrArgument<'a>,
        signature: impl CStrArgument<'a>,
        flags: VtableFlags,
        get: G,
        set: S,
    ) -> crate::Result<Self>
    where
        G: Fn(&mut MessageRef) -> super::Result<()> + Send + Sync + 'static,
        S: Fn(&mut MessageIter<'_>) -> super::Result<()> + Send + Sync + 'static,
    {
        self.add(
            member,
            signature,
            "",
            flags,
            Callbacks::Property {
                get: Box::new(get),
                set: Some(Box::new(set)),
            },
        )
    }

    fn add<'a>(
        mut self,
        member: impl CStrArgument<'a>,
        signature: impl CStrArgument<'a>,
        result: impl CStrArgument<'a>,
        flags: VtableFlags,
        callbacks: Callbacks,
    ) -> crate::Result<Self> {
        let member = member.try_into_cstr()?.into_owned();
        MemberName::from_bytes(member.as_bytes_with_nul())?;
        if self.entries.iter().any(|e| {
            e.member == member
                && std::mem::discriminant(&e.callbacks) == std::mem::discriminant(&callbacks)
        }) {
            return Err(invalid("duplicate vtable member"));
        }
        self.entries.push(Entry {
            member,
            signature: signature.try_into_cstr()?.into_owned(),
            result: result.try_into_cstr()?.into_owned(),
            flags,
            callbacks,
        });
        Ok(self)
    }

    pub(super) fn register(
        self,
        bus: &BusRef,
        path: &ObjectPath,
        interface: &InterfaceName,
    ) -> crate::Result<()> {
        let mut table = Vec::with_capacity(self.entries.len() + 2);
        table.push(sd_bus_vtable::start(0));
        table.extend(self.entries.iter().map(Entry::raw));
        table.push(sd_bus_vtable::end());
        let registration = Box::into_raw(Box::new(Registration {
            table,
            definition: self,
        }));
        let mut slot = ptr::null_mut();
        // SAFETY: registration owns the terminated table, all its strings, and
        // callbacks. It stays live during registration and native error cleanup.
        let result = super::ffi_result(unsafe {
            raw::sd_bus_add_object_vtable(
                bus.as_ptr(),
                &mut slot,
                path.as_ptr(),
                interface.as_ptr(),
                (*registration).table.as_ptr(),
                registration.cast(),
            )
        });
        if let Err(error) = result {
            drop(unsafe { Box::from_raw(registration) });
            return Err(error);
        }
        // Native slot teardown unregisters the table before dropping userdata.
        unsafe { super::float_userdata_slot(slot, registration) }
    }
}

fn invalid(error: impl std::fmt::Display) -> Error {
    Error::new(ErrorKind::InvalidInput, error.to_string())
}

struct Registration {
    table: Vec<sd_bus_vtable>,
    definition: Vtable,
}

#[test]
fn owned_vtable_strings_transfer_their_buffers() {
    let member = CString::new("Echo").unwrap();
    let signature = CString::new("s").unwrap();
    let result = CString::new("s").unwrap();
    let pointers = (member.as_ptr(), signature.as_ptr(), result.as_ptr());
    let table = Vtable::new()
        .method(member, signature, result, VtableFlags::empty(), |_| Ok(()))
        .unwrap();
    let entry = &table.entries[0];
    assert_eq!(
        (
            entry.member.as_ptr(),
            entry.signature.as_ptr(),
            entry.result.as_ptr()
        ),
        pointers
    );
}

// All native offsets are zero. Lookup selects a callback without creating
// mutable aliases to captured application state, including during reentry.
unsafe fn find<'a>(
    userdata: *mut c_void,
    member: *const c_char,
    property: bool,
) -> Option<&'a Entry> {
    let registration = &*userdata.cast::<Registration>();
    let member = CStr::from_ptr(member);
    registration.definition.entries.iter().find(|entry| {
        entry.member.as_c_str() == member
            && match entry.callbacks {
                Callbacks::Property { .. } => property,
                Callbacks::Method(_) => !property,
                Callbacks::Signal => false,
            }
    })
}

// SAFETY: error is null or points to an initialized, empty, writable error
// output which may take ownership of the callback's error allocation.
unsafe fn invoke(
    error: *mut raw::sd_bus_error,
    callback: impl FnOnce() -> super::Result<()>,
) -> c_int {
    match catch_unwind(AssertUnwindSafe(callback)) {
        Ok(Ok(())) => 1,
        Ok(Err(failure)) => {
            if !error.is_null() {
                unsafe { failure.move_into(error) };
            }
            -libc::EIO
        }
        Err(_) => -libc::EIO,
    }
}

unsafe extern "C" fn method(
    message: *mut raw::sd_bus_message,
    userdata: *mut c_void,
    error: *mut raw::sd_bus_error,
) -> c_int {
    let message = MessageRef::from_ptr_mut(message);
    let member = raw::sd_bus_message_get_member(message.as_ptr());
    let Some(entry) = find(userdata, member, false) else {
        return -libc::ENOENT;
    };
    let Callbacks::Method(callback) = &entry.callbacks else {
        return -libc::ENOENT;
    };
    invoke(error, || callback(message))
}

unsafe extern "C" fn get_property(
    _: *mut raw::sd_bus,
    _: *const c_char,
    _: *const c_char,
    property: *const c_char,
    reply: *mut raw::sd_bus_message,
    userdata: *mut c_void,
    error: *mut raw::sd_bus_error,
) -> c_int {
    let Some(entry) = find(userdata, property, true) else {
        return -libc::ENOENT;
    };
    let Callbacks::Property { get, .. } = &entry.callbacks else {
        return -libc::ENOENT;
    };
    invoke(error, || get(MessageRef::from_ptr_mut(reply)))
}

unsafe extern "C" fn set_property(
    _: *mut raw::sd_bus,
    _: *const c_char,
    _: *const c_char,
    property: *const c_char,
    value: *mut raw::sd_bus_message,
    userdata: *mut c_void,
    error: *mut raw::sd_bus_error,
) -> c_int {
    let Some(entry) = find(userdata, property, true) else {
        return -libc::ENOENT;
    };
    let Callbacks::Property { set: Some(set), .. } = &entry.callbacks else {
        return -libc::ENOENT;
    };
    // The native property dispatcher already entered the value's variant.
    // Do not use MessageRef::iter(), which rewinds to the whole message's start.
    let mut iter = MessageIter {
        raw: value,
        life: PhantomData,
        peeked_signature: String::new(),
    };
    invoke(error, || set(&mut iter))
}
