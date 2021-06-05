use libsystemd_sys as sd;

#[test]
fn raw_send() {
    let a = ["MESSAGE=raw rust-systemd send"];
    let v = [sd::ConstIovec {
        iov_base: a[0].as_ptr() as *const _,
        iov_len: a[0].len(),
    }];
    let r = unsafe { sd::journal::sd_journal_sendv(v.as_ptr(), v.len() as std::os::raw::c_int) };
    assert!(r >= 0);
}
