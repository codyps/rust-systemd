#![feature(phase)]
#![feature(macro_rules)]

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
        use log::{Logger,LogRecord,LogLevel,LogLocation};
        use std::fmt;

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

        pub fn log_(record: &LogRecord) {
            let LogLevel(lvl) = record.level;
            send([format!("PRIORITY={}", lvl).as_slice(),
                  format!("MESSAGE={}", record.args).as_slice(),
                  format!("CODE_LINE={}", record.line).as_slice(),
                  format!("CODE_FILE={}", record.file).as_slice(),
                  format!("CODE_FUNCTION={}", record.module_path).as_slice(),
            ]);
        }

        pub fn log(level: u32, loc: &'static LogLocation, args: &fmt::Arguments)
        {
            log_(&LogRecord {
                level: LogLevel(level),
                args: args,
                file: loc.file,
                module_path: loc.module_path,
                line: loc.line,
            });
        }

        pub struct JournalLogger;
        impl Logger for JournalLogger {
            fn log(&mut self, record: &LogRecord) {
                log_(record);
            }
        }
    }
}

/* This is nearly a clone of log!() except:
*   - it accepts a func argument to invoke (instead of hard coding ::log::log())
*   - it does not filter on log_enabled!()
*/
macro_rules! log_with(
    ($func:expr, $lvl:expr, $($arg:tt)+) => ({
        static LOC: ::log::LogLocation = ::log::LogLocation {
            line: line!(),
            file: file!(),
            module_path: module_path!()
        };
        let lvl = $lvl;
        let func = $func;
        format_args!(|args| { func(lvl, &LOC, args) }, $($arg)+)
    })
)

macro_rules! sd_journal_log(
    ($lvl:expr, $($arg:tt)+) => (log_with!(::systemd::journal::log, $lvl, $($arg)+))
)

#[test]
fn test() {
    use systemd::journal;
    use log::{set_logger};
    journal::send(["CODE_FILE=HI", "CODE_LINE=1213", "CODE_FUNCTION=LIES"]);
    journal::print(1, format!("Rust can talk to the journal: {}",
                              4i).as_slice());
    set_logger(box journal::JournalLogger);
    log!(0, "HI");
    sd_journal_log!(4, "HI {}", 2i);
}
