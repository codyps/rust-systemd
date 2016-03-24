use log::{self, Log, LogRecord, LogLocation, SetLoggerError};
use std::{fmt, ptr, result};
use std::collections::BTreeMap;
use ffi;
use super::Result;
use libc::{self, free, c_int, c_char, size_t};
use std::ffi::{CString, CStr};

/// Send preformatted fields to systemd.
///
/// This is a relatively low-level operation and probably not suitable unless
/// you need precise control over which fields are sent to systemd.
pub fn send(args: &[&str]) -> c_int {
    let iovecs = ffi::array_to_iovecs(args);
    unsafe { ffi::sd_journal_sendv(iovecs.as_ptr(), iovecs.len() as c_int) }
}

/// Send a simple message to systemd.
pub fn print(lvl: u32, s: &str) -> c_int {
    send(&[&format!("PRIORITY={}", lvl), &format!("MESSAGE={}", s)])
}

/// Send a `log::LogRecord` to systemd.
pub fn log_record(record: &LogRecord) {
    let lvl: usize = unsafe {
        use std::mem;
        mem::transmute(record.level())
    };
    log(lvl, record.location(), record.args());
}

pub fn log(level: usize, loc: &LogLocation, args: &fmt::Arguments) {
    send(&[&format!("PRIORITY={}", level),
           &format!("MESSAGE={}", args),
           &format!("CODE_LINE={}", loc.line()),
           &format!("CODE_FILE={}", loc.file()),
           &format!("CODE_FUNCTION={}", loc.module_path())]);
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
        log::set_logger(|_max_log_level| Box::new(JournalLog))
    }
}

pub type JournalRecord = BTreeMap<String, String>;

/// A cursor into the systemd journal.
///
/// Supports read, next, previous, and seek operations.
pub struct Journal {
    j: ffi::sd_journal,
    cursor: String,
}

/// Represents the set of journal files to read.
pub enum JournalFiles {
    /// The system-wide journal.
    System,
    /// The current user's journal.
    CurrentUser,
    /// Both the system-wide journal and the current user's journal.
    All,
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
            JournalFiles::All => 0,
        };

        let journal = Journal {
            j: ptr::null_mut(),
            cursor: "".to_string(),
        };
        sd_try!(ffi::sd_journal_open(&journal.j, flags));
        sd_try!(ffi::sd_journal_seek_head(journal.j));
        Ok(journal)
    }

    /// Read the next record from the journal. Returns `io::EndOfFile` if there
    /// are no more records to read.
    pub fn next_record(&mut self) -> Result<Option<JournalRecord>> {
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
                let mut name_value = field.splitn(2, '=');
                let name = name_value.next().unwrap();
                let value = name_value.next().unwrap();
                ret.insert(From::from(name), From::from(value));
            }
        }

        // update cursor
        let mut c_cursor: *mut c_char = ptr::null_mut();
        let cursor: String;
        if sd_try!(ffi::sd_journal_get_cursor(self.j, &mut c_cursor)) == 0 {
            unsafe {
                // You should use Cstr for memory allocated by C
                cursor = CStr::from_ptr(c_cursor as *const _)
                             .to_string_lossy()
                             .into_owned();
            }
            self.cursor = cursor;
            unsafe {
                free(c_cursor as *mut libc::c_void);
            }
        }
        Ok(Some(ret))
    }

    pub fn seek_cursor<S>(&self, position: S) -> Result<()>
        where S: Into<String>
    {
        let position = position.into();
        let c_position = CString::new(position.clone());
        // seeks to latest time if cursor string is invalid
        sd_try!(ffi::sd_journal_seek_cursor(self.j, c_position.unwrap().as_ptr() as *const _));
        Ok(())
    }
}

impl Iterator for Journal {
    type Item = (JournalRecord, String);

    fn next(&mut self) -> Option<(JournalRecord, String)> {
        let next_record = self.next_record().unwrap();
        let wait_time: u64 = 1 << 63;
        match next_record {
            Some(record) => Some((record, self.cursor.clone())),
            None => {
                let wait_time: u64 = 1 << 63;
                let w_ret: i32;
                unsafe {
                    w_ret = ffi::sd_journal_wait(self.j, wait_time);
                }
                if w_ret <= 0 {
                    None
                } else {
                    self.next()
                }
            }
        }
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
