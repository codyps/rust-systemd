#![feature(phase)]

extern crate libc;
#[phase(plugin,link)]
extern crate log;

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
        use log::{Logger,LogRecord,LogLevel};

        #[link(name = "systemd")]
        extern {
            /* printf() like variadic */
            fn sd_journal_print(priority : c_int, format : *const c_char, ...) -> c_int;
            fn sd_journal_sendv(iv : *const const_iovec, n : c_int) -> c_int;

            fn sd_journal_print_with_location(prio: c_int, file_ish: *const c_char,
                                              line_ish: *const c_char, func: *const c_char,
                                              fmt: *const c_char, ...) -> c_int;
            fn sd_journal_sendv_with_location(file_ish: *const c_char, line_ish: *const c_char,
                                              func: *const c_char, iv: *const const_iovec,
                                              n : c_int) -> c_int;
        }

        fn array_to_iovecs(args: &[&str]) -> Vec<const_iovec> {
            args.iter().map(|d| {
                const_iovec { iov_base: d.as_ptr() as *const c_void, iov_len: d.len() as size_t }
            }).collect()
        }

        pub fn send(args : &[&str]) -> c_int {
            let iovecs = array_to_iovecs(args);
            unsafe { sd_journal_sendv(iovecs.as_ptr(), iovecs.len() as c_int) }
        }

        pub fn print(lvl : uint, s : &str) -> c_int {
            send([
                 format!("PRIORITY={}", lvl).as_slice(),
                 format!("MESSAGE={}", s).as_slice()
            ])
        }

        pub struct JournalLogger;
        impl Logger for JournalLogger {
            fn log(&mut self, record: &LogRecord) {
                let LogLevel(lvl) = record.level;
                send([format!("PRIORITY={}", lvl).as_slice(),
                      format!("MESSAGE={}", record.args).as_slice(),
                      format!("CODE_LINE={}", record.line).as_slice(),
                      format!("CODE_FILE={}", record.file).as_slice(),
                      format!("MODULE_PATH={}", record.module_path).as_slice(),
                    ]);
            }
        }
    }
}


#[test]
fn test() {
    use systemd::journal;
    use log::{set_logger};
    journal::send(["CODE_FILE=HI", "CODE_LINE=1213", "CODE_FUNCTION=LIES"]);
    journal::print(1, format!("Rust can talk to the journal: {}",
                              4i).as_slice());
    set_logger(box journal::JournalLogger);
    log!(0, "HI");
}
