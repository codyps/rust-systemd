#![feature(phase)]
#![feature(macro_rules)]

extern crate collections;
extern crate libc;
#[phase(plugin,link)] extern crate log;

#[macro_export]
macro_rules! sd_try(
    ($e:expr) => ({
        let ret: i32;
        unsafe {
            ret = $e;
        }
        if ret < 0 {
            return Err(::std::io::IoError::from_errno(ret.abs() as uint, false));
        }
        ret
    })
)

pub mod ffi;
pub mod journal;

/* This is nearly a clone of log!() except:
*   - it accepts a func argument to invoke (instead of hard coding ::log::log())
*   - it does not filter on log_enabled!()
*/
#[macro_export]
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

#[macro_export]
macro_rules! sd_journal_log(
    ($lvl:expr, $($arg:tt)+) => (log_with!(::systemd::journal::log, $lvl, $($arg)+))
)
