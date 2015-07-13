use libc::{c_int,size_t};
use log::{self,Log,LogRecord,LogLocation,SetLoggerError};
use std::{fmt,ptr,result};
use std::collections::BTreeMap;
use ffi;
use super::Result;

/// Send preformatted fields to systemd.
///
/// This is a relatively low-level operation and probably not suitable unless
/// you need precise control over which fields are sent to systemd.
pub fn send(args : &[&str]) -> c_int {
    let iovecs = ffi::array_to_iovecs(args);
    unsafe { ffi::sd_journal_sendv(iovecs.as_ptr(), iovecs.len() as c_int) }
}

/// Send a simple message to systemd.
pub fn print(lvl : u32, s : &str) -> c_int {
    send(&[
         &format!("PRIORITY={}", lvl),
         &format!("MESSAGE={}", s)
    ])
}

/// Send a `log::LogRecord` to systemd.
pub fn log_record(record: &LogRecord) {
    let lvl: usize = unsafe {
        use std::mem;
        mem::transmute(record.level())
    };
    log(lvl, record.location(), record.args());
}

pub fn log(level: usize, loc: &LogLocation, args: &fmt::Arguments)
{
    send(&[&format!("PRIORITY={}", level),
        &format!("MESSAGE={}", args),
        &format!("CODE_LINE={}", loc.line()),
        &format!("CODE_FILE={}", loc.file()),
        &format!("CODE_FUNCTION={}", loc.module_path()),
    ]);
}

pub struct JournalLog;
impl Log for JournalLog {
    fn enabled(&self, _metadata: &log::LogMetadata) -> bool {
        true
    }

    fn log(&self, record: &LogRecord) {
        log_record(record);
    }
}

impl JournalLog {
    pub fn init() -> result::Result<(), SetLoggerError> {
        log::set_logger(|_max_log_level| {
            Box::new(JournalLog)
        })
    }
}

pub type JournalRecord = BTreeMap<String, String>;

/// A cursor into the systemd journal.
///
/// Supports read, next, previous, and seek operations.
pub struct Journal {
    j: ffi::sd_journal
}

/// Represents the set of journal files to read.
pub enum JournalFiles {
    /// The system-wide journal.
    System,
    /// The current user's journal.
    CurrentUser,
    /// Both the system-wide journal and the current user's journal.
    All
}

impl Journal {
    /// Open the systemd journal for reading.
    ///
    /// Params:
    ///
    /// * files: the set of journal files to read. If the calling process
    ///   doesn't have permission to read the system journal, a call to
    ///   `Journal::open` with `System` or `All` will succeed, but system
    ///   journal entries won't be included. This behavior is due to systemd.
    /// * runtime_only: if true, include only journal entries from the current
    ///   boot. If false, include all entries.
    /// * local_only: if true, include only journal entries originating from
    ///   localhost. If false, include all entries.
    pub fn open(files: JournalFiles, runtime_only: bool, local_only: bool) -> Result<Journal> {
        let mut flags: c_int = 0;
        if runtime_only {
            flags |= ffi::SD_JOURNAL_RUNTIME_ONLY;
        }
        if local_only {
            flags |= ffi::SD_JOURNAL_LOCAL_ONLY;
        }
        flags |= match files {
            JournalFiles::System => ffi::SD_JOURNAL_SYSTEM,
            JournalFiles::CurrentUser => ffi::SD_JOURNAL_CURRENT_USER,
            JournalFiles::All => 0
        };

        let journal = Journal { j: ptr::null_mut() };
        sd_try!(ffi::sd_journal_open(&journal.j, flags));
        sd_try!(ffi::sd_journal_seek_head(journal.j));
        Ok(journal)
    }

    /// Read the next record from the journal. Returns `io::EndOfFile` if there
    /// are no more records to read.
    pub fn next_record(&self) -> Result<Option<JournalRecord>> {
        if sd_try!(ffi::sd_journal_next(self.j)) == 0 {
            return Ok(None);
        }
        unsafe { ffi::sd_journal_restart_data(self.j) }

        let mut ret: JournalRecord = BTreeMap::new();

        let mut sz: size_t = 0;
        let data: *mut u8 = ptr::null_mut();
        while sd_try!(ffi::sd_journal_enumerate_data(self.j, &data, &mut sz)) > 0 {
            unsafe {
                let b = ::std::slice::from_raw_parts_mut(data, sz as usize);
                let field = ::std::str::from_utf8_unchecked(b);
                let mut name_value = field.splitn(1, '=');
                let name = name_value.next().unwrap();
                let value = name_value.next().unwrap();
                ret.insert(From::from(name), From::from(value));
            }
        }

        Ok(Some(ret))
    }
}

impl Drop for Journal {
    fn drop(&mut self) {
        if !self.j.is_null() {
            unsafe {
                ffi::sd_journal_close(self.j);
            }
        }
    }
}

