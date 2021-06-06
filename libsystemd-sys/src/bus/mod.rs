use super::event::sd_event;
use super::id128::sd_id128_t;
use super::ConstIovec;
use super::{c_char, c_int, c_uint, c_void, gid_t, pid_t, size_t, uid_t};

mod protocol;
pub mod vtable;
pub use self::protocol::*;
pub use self::vtable::sd_bus_vtable;

#[allow(non_camel_case_types)]
pub enum sd_bus {}
#[allow(non_camel_case_types)]
pub enum sd_bus_message {}
#[allow(non_camel_case_types)]
pub enum sd_bus_slot {}
#[allow(non_camel_case_types)]
pub enum sd_bus_creds {}
#[allow(non_camel_case_types)]
pub enum sd_bus_track {}

#[allow(non_camel_case_types)]
pub type sd_bus_message_handler_t = Option<
    unsafe extern "C" fn(
        m: *mut sd_bus_message,
        userdata: *mut c_void,
        ret_error: *mut sd_bus_error,
    ) -> c_int,
>;
#[allow(non_camel_case_types)]
pub type sd_bus_property_get_t = Option<
    unsafe extern "C" fn(
        bus: *mut sd_bus,
        path: *const c_char,
        interface: *const c_char,
        property: *const c_char,
        reply: *mut sd_bus_message,
        userdata: *mut c_void,
        ret_error: *mut sd_bus_error,
    ) -> c_int,
>;
#[allow(non_camel_case_types)]
pub type sd_bus_property_set_t = Option<
    unsafe extern "C" fn(
        bus: *mut sd_bus,
        path: *const c_char,
        interface: *const c_char,
        property: *const c_char,
        value: *mut sd_bus_message,
        userdata: *mut c_void,
        ret_error: *mut sd_bus_error,
    ) -> c_int,
>;
#[allow(non_camel_case_types)]
pub type sd_bus_object_find_t = Option<
    unsafe extern "C" fn(
        bus: *mut sd_bus,
        path: *const c_char,
        interface: *const c_char,
        userdata: *mut c_void,
        ret_found: *mut *mut c_void,
        ret_error: *mut sd_bus_error,
    ) -> c_int,
>;
#[allow(non_camel_case_types)]
pub type sd_bus_node_enumerator_t = Option<
    unsafe extern "C" fn(
        bus: *mut sd_bus,
        prefix: *const c_char,
        userdata: *mut c_void,
        ret_nodes: *mut *mut *mut c_char,
        ret_error: *mut sd_bus_error,
    ) -> c_int,
>;
#[allow(non_camel_case_types)]
pub type sd_bus_track_handler_t =
    Option<unsafe extern "C" fn(track: *mut sd_bus_track, userdata: *mut c_void) -> c_int>;

#[allow(non_camel_case_types)]
type sd_destroy_t = Option<unsafe extern "C" fn(userdata: *mut c_void)>;

#[allow(non_camel_case_types)]
pub type sd_bus_destroy_t = sd_destroy_t;

#[repr(C)]
pub struct sd_bus_error {
    pub name: *const c_char,
    pub message: *const c_char,
    pub need_free: c_int,
}

#[repr(C)]
pub struct sd_bus_error_map {
    pub name: *const c_char,
    pub code: c_int,
}

extern "C" {
    // Connections
    pub fn sd_bus_default(ret: *mut *mut sd_bus) -> c_int;
    pub fn sd_bus_default_user(ret: *mut *mut sd_bus) -> c_int;
    pub fn sd_bus_default_system(ret: *mut *mut sd_bus) -> c_int;

    pub fn sd_bus_open(ret: *mut *mut sd_bus) -> c_int;
    pub fn sd_bus_open_user(ret: *mut *mut sd_bus) -> c_int;
    pub fn sd_bus_open_system(ret: *mut *mut sd_bus) -> c_int;
    pub fn sd_bus_open_system_remote(ret: *mut *mut sd_bus, host: *const c_char) -> c_int;
    pub fn sd_bus_open_system_machine(ret: *mut *mut sd_bus, host: *const c_char) -> c_int;

    pub fn sd_bus_new(ret: *mut *mut sd_bus) -> c_int;

    pub fn sd_bus_set_address(bus: *mut sd_bus, address: *const c_char) -> c_int;
    pub fn sd_bus_set_fd(bus: *mut sd_bus, input_fd: c_int, output_fd: c_int) -> c_int;
    pub fn sd_bus_set_exec(
        bus: *mut sd_bus,
        path: *const c_char,
        argv: *const *mut c_char,
    ) -> c_int;
    pub fn sd_bus_get_address(bus: *mut sd_bus, address: *mut *const c_char) -> c_int;
    pub fn sd_bus_set_bus_client(bus: *mut sd_bus, b: c_int) -> c_int;
    pub fn sd_bus_is_bus_client(bus: *mut sd_bus) -> c_int;
    pub fn sd_bus_set_server(bus: *mut sd_bus, b: c_int, bus_id: sd_id128_t) -> c_int;
    pub fn sd_bus_is_server(bus: *mut sd_bus) -> c_int;
    pub fn sd_bus_set_anonymous(bus: *mut sd_bus, b: c_int) -> c_int;
    pub fn sd_bus_is_anonymous(bus: *mut sd_bus) -> c_int;
    pub fn sd_bus_set_trusted(bus: *mut sd_bus, b: c_int) -> c_int;
    pub fn sd_bus_is_trusted(bus: *mut sd_bus) -> c_int;
    pub fn sd_bus_set_monitor(bus: *mut sd_bus, b: c_int) -> c_int;
    pub fn sd_bus_is_monitor(bus: *mut sd_bus) -> c_int;
    pub fn sd_bus_set_description(bus: *mut sd_bus, description: *const c_char) -> c_int;
    pub fn sd_bus_get_description(bus: *mut sd_bus, description: *mut *const c_char) -> c_int;
    pub fn sd_bus_negotiate_creds(bus: *mut sd_bus, b: c_int, creds_mask: u64) -> c_int;
    pub fn sd_bus_negotiate_timestamp(bus: *mut sd_bus, b: c_int) -> c_int;
    pub fn sd_bus_negotiate_fds(bus: *mut sd_bus, b: c_int) -> c_int;
    pub fn sd_bus_can_send(bus: *mut sd_bus, typ: c_char) -> c_int;
    pub fn sd_bus_get_creds_mask(bus: *mut sd_bus, creds_mask: *mut u64) -> c_int;
    pub fn sd_bus_set_allow_interactive_authorization(bus: *mut sd_bus, b: c_int) -> c_int;
    pub fn sd_bus_get_allow_interactive_authorization(bus: *mut sd_bus) -> c_int;
    pub fn sd_bus_set_exit_on_disconnect(bus: *mut sd_bus, b: c_int) -> c_int;
    pub fn sd_bus_get_exit_on_disconnect(bus: *mut sd_bus) -> c_int;
    pub fn sd_bus_set_close_on_exit(bus: *mut sd_bus, b: c_int) -> c_int;
    pub fn sd_bus_get_close_on_exit(bus: *mut sd_bus) -> c_int;
    pub fn sd_bus_set_watch_bind(bus: *mut sd_bus, b: c_int) -> c_int;
    pub fn sd_bus_get_watch_bind(bus: *mut sd_bus) -> c_int;
    pub fn sd_bus_set_connected_signal(bus: *mut sd_bus, b: c_int) -> c_int;
    pub fn sd_bus_get_connected_signal(bus: *mut sd_bus) -> c_int;
    pub fn sd_bus_set_sender(bus: *mut sd_bus, sender: *const c_char) -> c_int;
    pub fn sd_bus_get_sender(bus: *mut sd_bus, ret: *mut *const c_char) -> c_int;

    pub fn sd_bus_start(ret: *mut sd_bus) -> c_int;

    pub fn sd_bus_try_close(bus: *mut sd_bus) -> c_int;
    pub fn sd_bus_close(bus: *mut sd_bus);

    pub fn sd_bus_ref(bus: *mut sd_bus) -> *mut sd_bus;
    pub fn sd_bus_unref(bus: *mut sd_bus) -> *mut sd_bus;
    pub fn sd_bus_close_unref(bus: *mut sd_bus) -> *mut sd_bus;
    pub fn sd_bus_flush_close_unref(bus: *mut sd_bus) -> *mut sd_bus;

    pub fn sd_bus_default_flush_close();

    pub fn sd_bus_is_open(bus: *mut sd_bus) -> c_int;
    pub fn sd_bus_is_ready(bus: *mut sd_bus) -> c_int;

    pub fn sd_bus_get_bus_id(bus: *mut sd_bus, id: *mut sd_id128_t) -> c_int;
    pub fn sd_bus_get_scope(bus: *mut sd_bus, scope: *mut *const c_char) -> c_int;
    pub fn sd_bus_get_tid(bus: *mut sd_bus, tid: *mut pid_t) -> c_int;
    pub fn sd_bus_get_owner_creds(
        bus: *mut sd_bus,
        creds_mask: u64,
        ret: *mut *mut sd_bus_creds,
    ) -> c_int;

    pub fn sd_bus_send(bus: *mut sd_bus, m: *mut sd_bus_message, cookie: *mut u64) -> c_int;
    pub fn sd_bus_send_to(
        bus: *mut sd_bus,
        m: *mut sd_bus_message,
        destination: *const c_char,
        cookie: *mut u64,
    ) -> c_int;
    pub fn sd_bus_call(
        bus: *mut sd_bus,
        m: *mut sd_bus_message,
        usec: u64,
        ret_error: *mut sd_bus_error,
        reply: *mut *mut sd_bus_message,
    ) -> c_int;
    pub fn sd_bus_call_async(
        bus: *mut sd_bus,
        slot: *mut *mut sd_bus_slot,
        m: *mut sd_bus_message,
        callback: sd_bus_message_handler_t,
        userdata: *mut c_void,
        usec: u64,
    ) -> c_int;

    pub fn sd_bus_get_fd(bus: *mut sd_bus) -> c_int;
    pub fn sd_bus_get_events(bus: *mut sd_bus) -> c_int;
    pub fn sd_bus_get_timeout(bus: *mut sd_bus, timeout_usec: *mut u64) -> c_int;
    pub fn sd_bus_process(bus: *mut sd_bus, r: *mut *mut sd_bus_message) -> c_int;
    pub fn sd_bus_process_priority(
        bus: *mut sd_bus,
        max_priority: i64,
        r: *mut *mut sd_bus_message,
    ) -> c_int;
    pub fn sd_bus_wait(bus: *mut sd_bus, timeout_usec: u64) -> c_int;
    pub fn sd_bus_flush(bus: *mut sd_bus) -> c_int;

    pub fn sd_bus_get_current_slot(bus: *mut sd_bus) -> *mut sd_bus_slot;
    pub fn sd_bus_get_current_message(bus: *mut sd_bus) -> *mut sd_bus_message;
    pub fn sd_bus_get_current_handler(bus: *mut sd_bus) -> sd_bus_message_handler_t;
    pub fn sd_bus_get_current_userdata(bus: *mut sd_bus) -> *mut c_void;

    pub fn sd_bus_attach_event(bus: *mut sd_bus, e: *mut sd_event, priority: c_int) -> c_int;
    pub fn sd_bus_detach_event(bus: *mut sd_bus) -> c_int;
    pub fn sd_bus_get_event(bus: *mut sd_bus) -> *mut sd_event;

    pub fn sd_bus_get_n_queued_read(bus: *mut sd_bus, ret: *mut u64) -> c_int;
    pub fn sd_bus_get_n_queued_write(bus: *mut sd_bus, ret: *mut u64) -> c_int;

    pub fn sd_bus_set_method_call_timeout(bus: *mut sd_bus, usec: u64) -> c_int;
    pub fn sd_bus_get_method_call_timeout(bus: *mut sd_bus, ret: *mut u64) -> c_int;

    pub fn sd_bus_add_filter(
        bus: *mut sd_bus,
        slot: *mut *mut sd_bus_slot,
        callback: sd_bus_message_handler_t,
        userdata: *mut c_void,
    ) -> c_int;
    pub fn sd_bus_add_match(
        bus: *mut sd_bus,
        slot: *mut *mut sd_bus_slot,
        match_: *const c_char,
        callback: sd_bus_message_handler_t,
        userdata: *mut c_void,
    ) -> c_int;
    pub fn sd_bus_add_object(
        bus: *mut sd_bus,
        slot: *mut *mut sd_bus_slot,
        path: *const c_char,
        callback: sd_bus_message_handler_t,
        userdata: *mut c_void,
    ) -> c_int;
    pub fn sd_bus_add_fallback(
        bus: *mut sd_bus,
        slot: *mut *mut sd_bus_slot,
        prefix: *const c_char,
        callback: sd_bus_message_handler_t,
        userdata: *mut c_void,
    ) -> c_int;
    pub fn sd_bus_add_object_vtable(
        bus: *mut sd_bus,
        slot: *mut *mut sd_bus_slot,
        path: *const c_char,
        interface: *const c_char,
        vtable: *const sd_bus_vtable,
        userdata: *mut c_void,
    ) -> c_int;
    pub fn sd_bus_add_fallback_vtable(
        bus: *mut sd_bus,
        slot: *mut *mut sd_bus_slot,
        prefix: *const c_char,
        interface: *const c_char,
        vtable: *const sd_bus_vtable,
        find: sd_bus_object_find_t,
        userdata: *mut c_void,
    ) -> c_int;
    pub fn sd_bus_add_node_enumerator(
        bus: *mut sd_bus,
        slot: *mut *mut sd_bus_slot,
        path: *const c_char,
        callback: sd_bus_node_enumerator_t,
        userdata: *mut c_void,
    ) -> c_int;
    pub fn sd_bus_add_object_manager(
        bus: *mut sd_bus,
        slot: *mut *mut sd_bus_slot,
        path: *const c_char,
    ) -> c_int;

    // Slot object

    pub fn sd_bus_slot_ref(slot: *mut sd_bus_slot) -> *mut sd_bus_slot;
    pub fn sd_bus_slot_unref(slot: *mut sd_bus_slot) -> *mut sd_bus_slot;

    pub fn sd_bus_slot_get_bus(slot: *mut sd_bus_slot) -> *mut sd_bus;
    pub fn sd_bus_slot_get_userdata(slot: *mut sd_bus_slot) -> *mut c_void;
    pub fn sd_bus_slot_set_userdata(slot: *mut sd_bus_slot, userdata: *mut c_void) -> *mut c_void;
    pub fn sd_bus_slot_set_description(slot: *mut sd_bus_slot, description: *const c_char)
        -> c_int;
    pub fn sd_bus_slot_get_description(
        slot: *mut sd_bus_slot,
        description: *mut *const c_char,
    ) -> c_int;
    pub fn sd_bus_slot_get_floating(slot: *mut sd_bus_slot) -> c_int;
    pub fn sd_bus_slot_set_floating(slot: *mut sd_bus_slot, b: c_int) -> c_int;
    // since v239
    pub fn sd_bus_slot_set_destroy_callback(
        slot: *mut sd_bus_slot,
        callback: sd_bus_destroy_t,
    ) -> c_int;
    // since v239
    pub fn sd_bus_slot_get_destroy_callback(
        slot: *mut sd_bus_slot,
        callback: *mut sd_bus_destroy_t,
    ) -> c_int;

    pub fn sd_bus_slot_get_current_message(slot: *mut sd_bus_slot) -> *mut sd_bus_message;
    pub fn sd_bus_slot_get_current_handler(bus: *mut sd_bus_slot) -> sd_bus_message_handler_t;
    pub fn sd_bus_slot_get_current_userdata(slot: *mut sd_bus_slot) -> *mut c_void;

    // Message object

    pub fn sd_bus_message_new(bus: *mut sd_bus, m: *mut *mut sd_bus_message, typ: u8) -> c_int;
    pub fn sd_bus_message_new_signal(
        bus: *mut sd_bus,
        m: *mut *mut sd_bus_message,
        path: *const c_char,
        interface: *const c_char,
        member: *const c_char,
    ) -> c_int;
    pub fn sd_bus_message_new_method_call(
        bus: *mut sd_bus,
        m: *mut *mut sd_bus_message,
        destination: *const c_char,
        path: *const c_char,
        interface: *const c_char,
        member: *const c_char,
    ) -> c_int;
    pub fn sd_bus_message_new_method_return(
        call: *mut sd_bus_message,
        m: *mut *mut sd_bus_message,
    ) -> c_int;
    pub fn sd_bus_message_new_method_error(
        call: *mut sd_bus_message,
        m: *mut *mut sd_bus_message,
        e: *const sd_bus_error,
    ) -> c_int;
    pub fn sd_bus_message_new_method_errorf(
        call: *mut sd_bus_message,
        m: *mut *mut sd_bus_message,
        name: *const c_char,
        format: *const c_char,
        ...
    ) -> c_int;
    pub fn sd_bus_message_new_method_errno(
        call: *mut sd_bus_message,
        m: *mut *mut sd_bus_message,
        error: c_int,
        e: *const sd_bus_error,
    ) -> c_int;
    pub fn sd_bus_message_new_method_errnof(
        call: *mut sd_bus_message,
        m: *mut *mut sd_bus_message,
        error: c_int,
        format: *const c_char,
        ...
    ) -> c_int;

    pub fn sd_bus_message_ref(m: *mut sd_bus_message) -> *mut sd_bus_message;
    pub fn sd_bus_message_unref(m: *mut sd_bus_message) -> *mut sd_bus_message;

    pub fn sd_bus_message_seal(m: *mut sd_bus_message, cookie: u64, timeout_usec: u64) -> c_int;

    pub fn sd_bus_message_get_type(m: *mut sd_bus_message, typ: *mut u8) -> c_int;
    pub fn sd_bus_message_get_cookie(m: *mut sd_bus_message, cookie: *mut u64) -> c_int;
    pub fn sd_bus_message_get_reply_cookie(m: *mut sd_bus_message, cookie: *mut u64) -> c_int;
    pub fn sd_bus_message_get_priority(m: *mut sd_bus_message, priority: *mut i64) -> c_int;

    pub fn sd_bus_message_get_expect_reply(m: *mut sd_bus_message) -> c_int;
    pub fn sd_bus_message_get_auto_start(m: *mut sd_bus_message) -> c_int;
    pub fn sd_bus_message_get_allow_interactive_authorization(m: *mut sd_bus_message) -> c_int;

    pub fn sd_bus_message_get_signature(m: *mut sd_bus_message, complete: c_int) -> *const c_char;
    pub fn sd_bus_message_get_path(m: *mut sd_bus_message) -> *const c_char;
    pub fn sd_bus_message_get_interface(m: *mut sd_bus_message) -> *const c_char;
    pub fn sd_bus_message_get_member(m: *mut sd_bus_message) -> *const c_char;
    pub fn sd_bus_message_get_destination(m: *mut sd_bus_message) -> *const c_char;
    pub fn sd_bus_message_get_sender(m: *mut sd_bus_message) -> *const c_char;
    pub fn sd_bus_message_get_error(m: *mut sd_bus_message) -> *const sd_bus_error;
    pub fn sd_bus_message_get_errno(m: *mut sd_bus_message) -> c_int;

    pub fn sd_bus_message_get_monotonic_usec(m: *mut sd_bus_message, usec: *mut u64) -> c_int;
    pub fn sd_bus_message_get_realtime_usec(m: *mut sd_bus_message, usec: *mut u64) -> c_int;
    pub fn sd_bus_message_get_seqnum(m: *mut sd_bus_message, seqnum: *mut u64) -> c_int;

    pub fn sd_bus_message_get_bus(m: *mut sd_bus_message) -> *mut sd_bus;
    /// do not unref the result
    pub fn sd_bus_message_get_creds(m: *mut sd_bus_message) -> *mut sd_bus_creds;

    pub fn sd_bus_message_is_signal(
        m: *mut sd_bus_message,
        interface: *const c_char,
        member: *const c_char,
    ) -> c_int;
    pub fn sd_bus_message_is_method_call(
        m: *mut sd_bus_message,
        interface: *const c_char,
        member: *const c_char,
    ) -> c_int;
    pub fn sd_bus_message_is_method_error(m: *mut sd_bus_message, name: *const c_char) -> c_int;
    pub fn sd_bus_message_is_empty(m: *mut sd_bus_message) -> c_int;
    pub fn sd_bus_message_has_signature(m: *mut sd_bus_message, signature: *const c_char) -> c_int;

    pub fn sd_bus_message_set_expect_reply(m: *mut sd_bus_message, b: c_int) -> c_int;
    pub fn sd_bus_message_set_auto_start(m: *mut sd_bus_message, b: c_int) -> c_int;
    pub fn sd_bus_message_set_allow_interactive_authorization(
        m: *mut sd_bus_message,
        b: c_int,
    ) -> c_int;

    pub fn sd_bus_message_set_destination(
        m: *mut sd_bus_message,
        destination: *const c_char,
    ) -> c_int;
    pub fn sd_bus_message_set_sender(m: *mut sd_bus_message, sender: *const c_char) -> c_int;
    pub fn sd_bus_message_set_priority(m: *mut sd_bus_message, priority: i64) -> c_int;

    pub fn sd_bus_message_append(m: *mut sd_bus_message, types: *const c_char, ...) -> c_int;
    // pub fn sd_bus_message_appendv(m: *mut sd_bus_message, types: *const c_char, ap: va_list) ->
    // c_int;
    pub fn sd_bus_message_append_basic(
        m: *mut sd_bus_message,
        typ: c_char,
        p: *const c_void,
    ) -> c_int;
    pub fn sd_bus_message_append_array(
        m: *mut sd_bus_message,
        typ: c_char,
        ptr: *const c_void,
        size: size_t,
    ) -> c_int;
    pub fn sd_bus_message_append_array_space(
        m: *mut sd_bus_message,
        typ: c_char,
        size: size_t,
        ptr: *mut *mut c_void,
    ) -> c_int;
    pub fn sd_bus_message_append_array_iovec(
        m: *mut sd_bus_message,
        typ: c_char,
        iov: *const ConstIovec,
        n: c_uint,
    ) -> c_int;
    pub fn sd_bus_message_append_array_memfd(
        m: *mut sd_bus_message,
        typ: c_char,
        memfd: c_int,
        offset: u64,
        size: u64,
    ) -> c_int;
    pub fn sd_bus_message_append_string_space(
        m: *mut sd_bus_message,
        size: size_t,
        s: *mut *mut c_char,
    ) -> c_int;
    pub fn sd_bus_message_append_string_iovec(
        m: *mut sd_bus_message,
        iov: *const ConstIovec,
        n: c_uint,
    ) -> c_int;
    pub fn sd_bus_message_append_string_memfd(
        m: *mut sd_bus_message,
        memfd: c_int,
        offset: u64,
        size: u64,
    ) -> c_int;
    pub fn sd_bus_message_append_strv(m: *mut sd_bus_message, l: *mut *mut c_char) -> c_int;
    pub fn sd_bus_message_open_container(
        m: *mut sd_bus_message,
        typ: c_char,
        contents: *const c_char,
    ) -> c_int;
    pub fn sd_bus_message_close_container(m: *mut sd_bus_message) -> c_int;
    pub fn sd_bus_message_copy(
        m: *mut sd_bus_message,
        source: *mut sd_bus_message,
        all: c_int,
    ) -> c_int;

    pub fn sd_bus_message_read(m: *mut sd_bus_message, types: *const c_char, ...) -> c_int;
    //pub fn sd_bus_message_readv(m: *mut sd_bus_message, types: *const c_char, ap: va_list);
    pub fn sd_bus_message_read_basic(m: *mut sd_bus_message, typ: c_char, p: *mut c_void) -> c_int;
    pub fn sd_bus_message_read_array(
        m: *mut sd_bus_message,
        typ: c_char,
        ptr: *mut *const c_void,
        size: *mut size_t,
    ) -> c_int;
    // free the result!
    pub fn sd_bus_message_read_strv(m: *mut sd_bus_message, l: *mut *mut *mut c_char) -> c_int;
    pub fn sd_bus_message_skip(m: *mut sd_bus_message, types: *const c_char) -> c_int;
    pub fn sd_bus_message_enter_container(
        m: *mut sd_bus_message,
        typ: c_char,
        contents: *const c_char,
    ) -> c_int;
    pub fn sd_bus_message_exit_container(m: *mut sd_bus_message) -> c_int;
    pub fn sd_bus_message_peek_type(
        m: *mut sd_bus_message,
        typ: *mut c_char,
        contents: *mut *const c_char,
    ) -> c_int;
    pub fn sd_bus_message_verify_type(
        m: *mut sd_bus_message,
        typ: c_char,
        contents: *const c_char,
    ) -> c_int;
    pub fn sd_bus_message_at_end(m: *mut sd_bus_message, complete: c_int) -> c_int;
    pub fn sd_bus_message_rewind(m: *mut sd_bus_message, complete: c_int) -> c_int;

    // Bus management

    pub fn sd_bus_get_unique_name(bus: *mut sd_bus, unique: *mut *const c_char) -> c_int;
    pub fn sd_bus_request_name(bus: *mut sd_bus, name: *const c_char, flags: u64) -> c_int;
    pub fn sd_bus_request_name_async(
        bus: *mut sd_bus,
        ret_slot: *mut *mut sd_bus_slot,
        name: *const c_char,
        flags: u64,
        callback: sd_bus_message_handler_t,
        userdata: *mut c_void,
    ) -> c_int;
    pub fn sd_bus_release_name(bus: *mut sd_bus, name: *const c_char) -> c_int;
    pub fn sd_bus_release_name_async(
        bus: *mut sd_bus,
        ret_slot: *mut *mut sd_bus_slot,
        name: *const c_char,
        callback: sd_bus_message_handler_t,
        userdata: *mut c_void,
    ) -> c_int;
    // free the results
    pub fn sd_bus_list_names(
        bus: *mut sd_bus,
        acquired: *mut *mut *mut c_char,
        activatable: *mut *mut *mut c_char,
    ) -> c_int;
    // unref the result!
    pub fn sd_bus_get_name_creds(
        bus: *mut sd_bus,
        name: *const c_char,
        mask: u64,
        creds: *mut *mut sd_bus_creds,
    ) -> c_int;
    pub fn sd_bus_get_name_machine_id(
        bus: *mut sd_bus,
        name: *const c_char,
        machine: *mut sd_id128_t,
    ) -> c_int;

    // Convenience calls

    pub fn sd_bus_call_method(
        bus: *mut sd_bus,
        destination: *const c_char,
        path: *const c_char,
        interface: *const c_char,
        member: *const c_char,
        ret_error: *mut sd_bus_error,
        reply: *mut *mut sd_bus_message,
        types: *const c_char,
        ...
    ) -> c_int;
    pub fn sd_bus_call_method_async(
        bus: *mut sd_bus,
        slot: *mut *mut sd_bus_slot,
        destination: *const c_char,
        path: *const c_char,
        interface: *const c_char,
        member: *const c_char,
        callback: sd_bus_message_handler_t,
        userdata: *mut c_void,
        types: *const c_char,
        ...
    ) -> c_int;
    pub fn sd_bus_get_property(
        bus: *mut sd_bus,
        destination: *const c_char,
        path: *const c_char,
        interface: *const c_char,
        member: *const c_char,
        ret_error: *mut sd_bus_error,
        reply: *mut *mut sd_bus_message,
        typ: *const c_char,
    ) -> c_int;
    pub fn sd_bus_get_property_trivial(
        bus: *mut sd_bus,
        destination: *const c_char,
        path: *const c_char,
        interface: *const c_char,
        member: *const c_char,
        ret_error: *mut sd_bus_error,
        typ: c_char,
        ret_ptr: *mut c_void,
    ) -> c_int;
    /// free the result!
    pub fn sd_bus_get_property_string(
        bus: *mut sd_bus,
        destination: *const c_char,
        path: *const c_char,
        interface: *const c_char,
        member: *const c_char,
        ret_error: *mut sd_bus_error,
        ret: *mut *mut c_char,
    ) -> c_int;
    /// free the result!
    pub fn sd_bus_get_property_strv(
        bus: *mut sd_bus,
        destination: *const c_char,
        path: *const c_char,
        interface: *const c_char,
        member: *const c_char,
        ret_error: *mut sd_bus_error,
        ret: *mut *mut *mut c_char,
    ) -> c_int;
    pub fn sd_bus_set_property(
        bus: *mut sd_bus,
        destination: *const c_char,
        path: *const c_char,
        interface: *const c_char,
        member: *const c_char,
        ret_error: *mut sd_bus_error,
        typ: *const c_char,
        ...
    ) -> c_int;

    pub fn sd_bus_reply_method_return(
        call: *mut sd_bus_message,
        types: *const c_char,
        ...
    ) -> c_int;
    pub fn sd_bus_reply_method_error(call: *mut sd_bus_message, e: *const sd_bus_error) -> c_int;
    pub fn sd_bus_reply_method_errorf(
        call: *mut sd_bus_message,
        name: *const c_char,
        format: *const c_char,
        ...
    ) -> c_int;
    pub fn sd_bus_reply_method_errno(
        call: *mut sd_bus_message,
        error: c_int,
        e: *const sd_bus_error,
    ) -> c_int;
    pub fn sd_bus_reply_method_errnof(
        call: *mut sd_bus_message,
        error: c_int,
        format: *const c_char,
        ...
    ) -> c_int;

    pub fn sd_bus_emit_signal(
        bus: *mut sd_bus,
        path: *const c_char,
        interface: *const c_char,
        member: *const c_char,
        types: *const c_char,
        ...
    ) -> c_int;

    pub fn sd_bus_emit_properties_changed_strv(
        bus: *mut sd_bus,
        path: *const c_char,
        interface: *const c_char,
        names: *mut *mut c_char,
    ) -> c_int;
    pub fn sd_bus_emit_properties_changed(
        bus: *mut sd_bus,
        path: *const c_char,
        interface: *const c_char,
        name: *const c_char,
        ...
    ) -> c_int;

    pub fn sd_bus_emit_object_added(bus: *mut sd_bus, path: *const c_char) -> c_int;
    pub fn sd_bus_emit_object_removed(bus: *mut sd_bus, path: *const c_char) -> c_int;
    pub fn sd_bus_emit_interfaces_added_strv(
        bus: *mut sd_bus,
        path: *const c_char,
        interfaces: *mut *mut c_char,
    ) -> c_int;
    pub fn sd_bus_emit_interfaces_added(
        bus: *mut sd_bus,
        path: *const c_char,
        interface: *const c_char,
        ...
    ) -> c_int;
    pub fn sd_bus_emit_interfaces_removed_strv(
        bus: *mut sd_bus,
        path: *const c_char,
        interfaces: *mut *mut c_char,
    ) -> c_int;
    pub fn sd_bus_emit_interfaces_removed(
        bus: *mut sd_bus,
        path: *const c_char,
        interface: *const c_char,
        ...
    ) -> c_int;

    pub fn sd_bus_query_sender_creds(
        call: *mut sd_bus_message,
        mask: u64,
        creds: *mut *mut sd_bus_creds,
    ) -> c_int;
    pub fn sd_bus_query_sender_privilege(call: *mut sd_bus_message, capability: c_int) -> c_int;

    // Credential handling

    pub fn sd_bus_creds_new_from_pid(
        ret: *mut *mut sd_bus_creds,
        pid: pid_t,
        creds_mask: u64,
    ) -> c_int;
    pub fn sd_bus_creds_ref(c: *mut sd_bus_creds) -> *mut sd_bus_creds;
    pub fn sd_bus_creds_unref(c: *mut sd_bus_creds) -> *mut sd_bus_creds;
    pub fn sd_bus_creds_get_mask(c: *const sd_bus_creds) -> u64;
    pub fn sd_bus_creds_get_augmented_mask(c: *const sd_bus_creds) -> u64;

    pub fn sd_bus_creds_get_pid(c: *mut sd_bus_creds, pid: *mut pid_t) -> c_int;
    pub fn sd_bus_creds_get_ppid(c: *mut sd_bus_creds, ppid: *mut pid_t) -> c_int;
    pub fn sd_bus_creds_get_tid(c: *mut sd_bus_creds, tid: *mut pid_t) -> c_int;
    pub fn sd_bus_creds_get_uid(c: *mut sd_bus_creds, uid: *mut uid_t) -> c_int;
    pub fn sd_bus_creds_get_euid(c: *mut sd_bus_creds, euid: *mut uid_t) -> c_int;
    pub fn sd_bus_creds_get_suid(c: *mut sd_bus_creds, suid: *mut uid_t) -> c_int;
    pub fn sd_bus_creds_get_fsuid(c: *mut sd_bus_creds, fsuid: *mut uid_t) -> c_int;
    pub fn sd_bus_creds_get_gid(c: *mut sd_bus_creds, gid: *mut gid_t) -> c_int;
    pub fn sd_bus_creds_get_egid(c: *mut sd_bus_creds, egid: *mut gid_t) -> c_int;
    pub fn sd_bus_creds_get_sgid(c: *mut sd_bus_creds, sgid: *mut gid_t) -> c_int;
    pub fn sd_bus_creds_get_fsgid(c: *mut sd_bus_creds, fsgid: *mut gid_t) -> c_int;
    pub fn sd_bus_creds_get_supplementary_gids(
        c: *mut sd_bus_creds,
        gids: *const *mut gid_t,
    ) -> c_int;
    pub fn sd_bus_creds_get_comm(c: *mut sd_bus_creds, comm: *mut *const c_char) -> c_int;
    pub fn sd_bus_creds_get_tid_comm(c: *mut sd_bus_creds, comm: *mut *const c_char) -> c_int;
    pub fn sd_bus_creds_get_exe(c: *mut sd_bus_creds, exe: *mut *const c_char) -> c_int;
    pub fn sd_bus_creds_get_cmdline(c: *mut sd_bus_creds, cmdline: *mut *mut *mut c_char) -> c_int;
    pub fn sd_bus_creds_get_cgroup(c: *mut sd_bus_creds, cgroup: *mut *const c_char) -> c_int;
    pub fn sd_bus_creds_get_unit(c: *mut sd_bus_creds, unit: *mut *const c_char) -> c_int;
    pub fn sd_bus_creds_get_slice(c: *mut sd_bus_creds, slice: *mut *const c_char) -> c_int;
    pub fn sd_bus_creds_get_user_unit(c: *mut sd_bus_creds, unit: *mut *const c_char) -> c_int;
    pub fn sd_bus_creds_get_user_slice(c: *mut sd_bus_creds, slice: *mut *const c_char) -> c_int;
    pub fn sd_bus_creds_get_session(c: *mut sd_bus_creds, session: *mut *const c_char) -> c_int;
    pub fn sd_bus_creds_get_owner_uid(c: *mut sd_bus_creds, uid: *mut uid_t) -> c_int;
    pub fn sd_bus_creds_has_effective_cap(c: *mut sd_bus_creds, capability: c_int) -> c_int;
    pub fn sd_bus_creds_has_permitted_cap(c: *mut sd_bus_creds, capability: c_int) -> c_int;
    pub fn sd_bus_creds_has_inheritable_cap(c: *mut sd_bus_creds, capability: c_int) -> c_int;
    pub fn sd_bus_creds_has_bounding_cap(c: *mut sd_bus_creds, capability: c_int) -> c_int;
    pub fn sd_bus_creds_get_selinux_context(
        c: *mut sd_bus_creds,
        context: *mut *const c_char,
    ) -> c_int;
    pub fn sd_bus_creds_get_audit_session_id(c: *mut sd_bus_creds, sessionid: *mut u32) -> c_int;
    pub fn sd_bus_creds_get_audit_login_uid(c: *mut sd_bus_creds, loginuid: *mut uid_t) -> c_int;
    pub fn sd_bus_creds_get_tty(c: *mut sd_bus_creds, tty: *mut *const c_char) -> c_int;
    pub fn sd_bus_creds_get_unique_name(c: *mut sd_bus_creds, name: *mut *const c_char) -> c_int;
    pub fn sd_bus_creds_get_well_known_names(
        c: *mut sd_bus_creds,
        names: *mut *mut *mut c_char,
    ) -> c_int;
    pub fn sd_bus_creds_get_description(c: *mut sd_bus_creds, name: *mut *const c_char) -> c_int;

    // Error structures

    pub fn sd_bus_error_free(e: *mut sd_bus_error);
    pub fn sd_bus_error_set(
        e: *mut sd_bus_error,
        name: *const c_char,
        message: *const c_char,
    ) -> c_int;
    pub fn sd_bus_error_setf(
        e: *mut sd_bus_error,
        name: *const c_char,
        format: *const c_char,
        ...
    ) -> c_int;
    pub fn sd_bus_error_set_const(
        e: *mut sd_bus_error,
        name: *const c_char,
        message: *const c_char,
    ) -> c_int;
    pub fn sd_bus_error_set_errno(e: *mut sd_bus_error, error: c_int) -> c_int;
    pub fn sd_bus_error_set_errnof(
        e: *mut sd_bus_error,
        error: c_int,
        format: *const c_char,
        ...
    ) -> c_int;

    pub fn sd_bus_error_get_errno(e: *const sd_bus_error) -> c_int;
    pub fn sd_bus_error_copy(dest: *mut sd_bus_error, e: *const sd_bus_error) -> c_int;
    pub fn sd_bus_error_is_set(e: *const sd_bus_error) -> c_int;
    pub fn sd_bus_error_has_name(e: *const sd_bus_error, name: *const c_char) -> c_int;

    pub fn sd_bus_error_add_map(map: *const sd_bus_error_map) -> c_int;

    // Label escaping

    pub fn sd_bus_path_encode(
        prefix: *const c_char,
        external_id: *const c_char,
        ret_path: *mut *mut c_char,
    ) -> c_int;
    pub fn sd_bus_path_encode_many(
        out: *mut *mut c_char,
        path_template: *const c_char,
        ...
    ) -> c_int;
    pub fn sd_bus_path_decode(
        path: *const c_char,
        prefix: *const c_char,
        ret_external_id: *mut *mut c_char,
    ) -> c_int;
    pub fn sd_bus_path_decode_many(path: *const c_char, path_template: *const c_char, ...)
        -> c_int;

    // Tracking peers

    pub fn sd_bus_track_new(
        bus: *mut sd_bus,
        track: *mut *mut sd_bus_track,
        handler: sd_bus_track_handler_t,
        userdata: *mut c_void,
    ) -> c_int;
    pub fn sd_bus_track_ref(track: *mut sd_bus_track) -> *mut sd_bus_track;
    pub fn sd_bus_track_unref(track: *mut sd_bus_track) -> *mut sd_bus_track;

    pub fn sd_bus_track_get_bus(track: *mut sd_bus_track) -> *mut sd_bus;
    pub fn sd_bus_track_get_userdata(track: *mut sd_bus_track) -> *mut c_void;
    pub fn sd_bus_track_set_userdata(
        track: *mut sd_bus_track,
        userdata: *mut c_void,
    ) -> *mut c_void;

    pub fn sd_bus_track_add_sender(track: *mut sd_bus_track, m: *mut sd_bus_message) -> c_int;
    pub fn sd_bus_track_remove_sender(track: *mut sd_bus_track, m: *mut sd_bus_message) -> c_int;
    pub fn sd_bus_track_add_name(track: *mut sd_bus_track, name: *const c_char) -> c_int;
    pub fn sd_bus_track_remove_name(track: *mut sd_bus_track, name: *const c_char) -> c_int;

    pub fn sd_bus_track_set_recursive(track: *mut sd_bus_track, b: c_int) -> c_int;
    pub fn sd_bus_track_count_sender(track: *mut sd_bus_track, m: *mut sd_bus_message) -> c_int;
    pub fn sd_bus_track_count_name(track: *mut sd_bus_track, name: *const c_char) -> c_int;

    pub fn sd_bus_track_count(track: *mut sd_bus_track) -> c_uint;
    pub fn sd_bus_track_contains(track: *mut sd_bus_track, names: *const c_char) -> *const c_char;
    pub fn sd_bus_track_first(track: *mut sd_bus_track) -> *const c_char;
    pub fn sd_bus_track_next(track: *mut sd_bus_track) -> *const c_char;

    pub fn sd_bus_track_set_destroy_callback(
        track: *mut sd_bus_track,
        callback: sd_bus_destroy_t,
    ) -> c_int;
    pub fn sd_bus_track_get_destroy_callback(
        track: *mut sd_bus_track,
        ret: *mut sd_bus_destroy_t,
    ) -> c_int;
}
