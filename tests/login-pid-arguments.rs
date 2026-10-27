use libc::{c_char, c_int, pid_t, uid_t};
use std::{ffi::CStr, ptr};
use systemd::login::{self, UnitType};

type NativeStringQuery = unsafe extern "C" fn(pid_t, *mut *mut c_char) -> c_int;

fn native_string(query: NativeStringQuery, pid: pid_t) -> Result<String, i32> {
    let mut output = ptr::null_mut();
    // SAFETY: output is writable and lives until the native call returns.
    let r = unsafe { query(pid, &mut output) };
    if r < 0 {
        assert!(output.is_null());
        return Err(-r);
    }
    assert!(!output.is_null());
    // SAFETY: successful sd_pid_get_* calls return a malloc-allocated C string.
    let value = unsafe { CStr::from_ptr(output) }
        .to_string_lossy()
        .into_owned();
    // SAFETY: the string is owned by this call and has been copied above.
    unsafe { libc::free(output.cast()) };
    Ok(value)
}

fn native_owner_uid(pid: pid_t) -> Result<uid_t, i32> {
    let mut uid = 0;
    // SAFETY: uid is writable and lives until the native call returns.
    let r = unsafe { libsystemd_sys::login::sd_pid_get_owner_uid(pid, &mut uid) };
    if r < 0 {
        Err(-r)
    } else {
        Ok(uid)
    }
}

fn outcome<T>(result: systemd::Result<T>) -> Result<T, i32> {
    result.map_err(|e| e.raw_os_error().unwrap())
}

#[test]
fn pid_argument_forms_match_native() {
    macro_rules! check {
        ($native:ident, $query:ident, $alias:ident $(, $kind:ident)?) => {{
            let native = libsystemd_sys::login::$native;
            let current = native_string(native, 0);
            assert_eq!(outcome(login::$query($(UnitType::$kind,)? None)), current);
            assert_eq!(outcome(login::$alias($(UnitType::$kind,)? None)), current);

            // Include current-process selection, an explicit PID, and native error cases.
            for pid in [0, 1, -1, pid_t::MAX] {
                let expected = native_string(native, pid);
                assert_eq!(outcome(login::$query($(UnitType::$kind,)? pid)), expected,
                    "{}({pid})", stringify!($query));
                assert_eq!(outcome(login::$query($(UnitType::$kind,)? Some(pid))), expected,
                    "{}(Some({pid}))", stringify!($query));
                assert_eq!(outcome(login::$alias($(UnitType::$kind,)? pid)), expected,
                    "{}({pid})", stringify!($alias));
                assert_eq!(outcome(login::$alias($(UnitType::$kind,)? Some(pid))), expected,
                    "{}(Some({pid}))", stringify!($alias));
                if pid == -1 {
                    assert_eq!(expected, Err(libc::EINVAL));
                }
            }
        }};
    }

    check!(sd_pid_get_unit, get_unit, process_unit, SystemUnit);
    check!(sd_pid_get_user_unit, get_unit, process_unit, UserUnit);
    check!(sd_pid_get_slice, get_slice, process_slice, SystemUnit);
    check!(sd_pid_get_user_slice, get_slice, process_slice, UserUnit);
    check!(
        sd_pid_get_machine_name,
        get_machine_name,
        process_machine_name
    );
    check!(sd_pid_get_cgroup, get_cgroup, process_cgroup);
    check!(sd_pid_get_session, get_session, process_session);

    let current = native_owner_uid(0);
    assert_eq!(outcome(login::get_owner_uid(None)), current);
    assert_eq!(outcome(login::process_owner_uid(None)), current);
    for pid in [0, 1, -1, pid_t::MAX] {
        let expected = native_owner_uid(pid);
        assert_eq!(outcome(login::get_owner_uid(pid)), expected);
        assert_eq!(outcome(login::get_owner_uid(Some(pid))), expected);
        assert_eq!(outcome(login::process_owner_uid(pid)), expected);
        assert_eq!(outcome(login::process_owner_uid(Some(pid))), expected);
        if pid == -1 {
            assert_eq!(expected, Err(libc::EINVAL));
        }
    }
}

#[test]
fn pid_queries_preserve_typed_function_pointer_calls() {
    type StringQuery = fn(Option<pid_t>) -> systemd::Result<String>;
    type UnitQuery = fn(UnitType, Option<pid_t>) -> systemd::Result<String>;
    type UidQuery = fn(Option<pid_t>) -> systemd::Result<uid_t>;

    let string_queries: [StringQuery; 6] = [
        login::get_machine_name,
        login::process_machine_name,
        login::get_cgroup,
        login::process_cgroup,
        login::get_session,
        login::process_session,
    ];
    for query in string_queries {
        assert_eq!(outcome(query(Some(-1))), Err(libc::EINVAL));
    }
    let unit_queries: [UnitQuery; 4] = [
        login::get_unit,
        login::process_unit,
        login::get_slice,
        login::process_slice,
    ];
    for query in unit_queries {
        assert_eq!(
            outcome(query(UnitType::SystemUnit, Some(-1))),
            Err(libc::EINVAL)
        );
        assert_eq!(
            outcome(query(UnitType::UserUnit, Some(-1))),
            Err(libc::EINVAL)
        );
    }
    let uid_queries: [UidQuery; 2] = [login::get_owner_uid, login::process_owner_uid];
    for query in uid_queries {
        assert_eq!(outcome(query(Some(-1))), Err(libc::EINVAL));
    }
}
