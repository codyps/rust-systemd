extern crate libc;

#[allow(dead_code)]
mod posix {
    use libc::{c_void,size_t};
    #[repr(C)]
    pub struct iovec {
        pub iov_base: *mut c_void,
        pub iov_len: size_t
    }

    #[repr(C)]
    pub struct const_iovec {
        pub iov_base: *const c_void,
        pub iov_len: size_t
    }
}

mod systemd {
    #[allow(dead_code)]
    pub mod journal {
        use libc::{c_char,c_int,c_void,size_t};
        use posix::const_iovec;
        #[link(name = "systemd")]
        extern {
            /* printf() like variadic */
            fn sd_journal_print(priority : c_int, format : *const c_char, ...) -> c_int;
            /* NULL terminated variadic */
            fn sd_journal_send(format : *const c_char, ...) -> c_int;
            fn sd_journal_sendv(iv : *const const_iovec, n : c_int) -> c_int;

            fn sd_journal_print_with_location(prio: c_int, file_ish: *const c_char,
                                              line_ish: *const c_char, func: *const c_char,
                                              fmt: *const c_char, ...);
        }

        pub fn print(lvl : uint, s : &str) -> int {
            s.with_c_str(|c_s| {
                unsafe { sd_journal_print(lvl as c_int, c_s) }
            }) as int
        }

        pub fn send(args : &[&str]) -> c_int {
            let iovecs: Vec<const_iovec> = args.iter().map(|d| {
                const_iovec { iov_base: d.as_ptr() as *const c_void, iov_len: d.len() as size_t }
            }).collect();
            unsafe { sd_journal_sendv(iovecs.as_ptr(), iovecs.len() as c_int) }
        }
    }
}

#[test]
fn test() {
    use systemd::journal;
    journal::print(1, "Rust can talk to the journal");
    journal::send(["CODE_FILE=HI", "CODE_LINE=1213", "CODE_FUNCTION=LIES"]);
}
