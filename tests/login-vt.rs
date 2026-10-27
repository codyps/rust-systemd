#![deny(deprecated)]

use libc::{c_char, c_int, c_uint};
use systemd::login;

// Isolate the output-pointer contract from the host's login sessions. This symbol
// overrides libsystemd's function only in this integration-test executable.
#[no_mangle]
unsafe extern "C" fn sd_session_get_vt(_session: *const c_char, vt: *mut c_uint) -> c_int {
    if vt.is_null() {
        return -libc::EINVAL;
    }
    // SAFETY: the caller supplies writable storage for one c_uint.
    unsafe { *vt = 7 };
    0
}

#[test]
fn get_vt_provides_output_storage() {
    assert_eq!(login::get_vt("test-session").unwrap(), 7);
}
