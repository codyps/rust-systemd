use libc::{c_char, c_int, size_t};
use log::{self, Log, LogRecord, LogLocation, LogLevelFilter, SetLoggerError};
use std::{fmt, io, ptr, result};
use std::collections::BTreeMap;
use std::io::ErrorKind::InvalidData;
use std::os::raw::c_void;
use ffi::array_to_iovecs;
use ffi::id128::sd_id128_t;
use ffi::journal as ffi;
use id128::Id128;
use super::Result;
use std::time;
use mbox::MString;

/// Send preformatted fields to systemd.
///
/// This is a relatively low-level operation and probably not suitable unless
/// you need precise control over which fields are sent to systemd.
pub fn send(args: &[&str]) -> c_int {
    let iovecs = array_to_iovecs(args);
    unsafe { ffi::sd_journal_sendv(iovecs.as_ptr(), iovecs.len() as c_int) }
}

/// Send a simple message to systemd-journald.
pub fn print(lvl: u32, s: &str) -> c_int {
    send(&[&format!("PRIORITY={}", lvl), &format!("MESSAGE={}", s)])
}

/// Send a `log::LogRecord` to systemd-journald.
pub fn log_record(record: &LogRecord) {
    let lvl: usize = unsafe {
        use std::mem;
        mem::transmute(record.level())
    };
    log(lvl, record.location(), record.args());
}

/// Record a log entry, with custom priority and location.
pub fn log(level: usize, loc: &LogLocation, args: &fmt::Arguments) {
    send(&[&format!("PRIORITY={}", level),
           &format!("MESSAGE={}", args),
           &format!("CODE_LINE={}", loc.line()),
           &format!("CODE_FILE={}", loc.file()),
           &format!("CODE_FUNCTION={}", loc.module_path())]);
}

/// Logger implementation over systemd-journald.
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
        Self::init_with_level(LogLevelFilter::Info)
    }

    pub fn init_with_level(level: LogLevelFilter) -> result::Result<(), SetLoggerError> {
        log::set_logger(|max_log_level| {
            max_log_level.set(level);
            Box::new(JournalLog)
        })
    }
}

fn duration_from_usec(usec: u64) -> time::Duration {
    let secs = usec / 1_000_000;
    let sub_usec = (usec % 1_000_000) as u32;
    let sub_nsec = sub_usec * 1000;
    time::Duration::new(secs, sub_nsec)
}

fn system_time_from_realtime_usec(usec: u64) -> time::SystemTime {
    let d = duration_from_usec(usec);
    time::UNIX_EPOCH + d
}

// A single log entry from journal.
pub type JournalRecord = BTreeMap<String, String>;

/// A reader for systemd journal.
///
/// Supports read, next, previous, and seek operations.
pub struct Journal {
    j: *mut ffi::sd_journal,
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

/// Seeking position in journal.
pub enum JournalSeek {
    Head,
    Current,
    Tail,
    ClockMonotonic {
        boot_id: Id128,
        usec: u64,
    },
    ClockRealtime {
        usec: u64,
    },
    Cursor {
        cursor: String,
    },
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

        let mut journal = Journal { j: ptr::null_mut() };
        sd_try!(ffi::sd_journal_open(&mut journal.j, flags));
        sd_try!(ffi::sd_journal_seek_head(journal.j));
        Ok(journal)
    }

    /// Read the next record from the journal. Returns `Ok(None)` if there
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
                let mut name_value = field.splitn(2, '=');
                let name = name_value.next().unwrap();
                let value = name_value.next().unwrap();
                ret.insert(From::from(name), From::from(value));
            }
        }

        Ok(Some(ret))
    }

    /// Seek to a specific position in journal. On success, returns a cursor
    /// to the current entry.
    pub fn seek(&self, seek: JournalSeek) -> Result<String> {
        match seek {
            JournalSeek::Head => sd_try!(ffi::sd_journal_seek_head(self.j)),
            JournalSeek::Current => 0,
            JournalSeek::Tail => sd_try!(ffi::sd_journal_seek_tail(self.j)),
            JournalSeek::ClockMonotonic { boot_id, usec } => {
                sd_try!(ffi::sd_journal_seek_monotonic_usec(self.j,
                                                            sd_id128_t {
                                                                bytes: *boot_id.as_bytes(),
                                                            },
                                                            usec))
            }
            JournalSeek::ClockRealtime { usec } => {
                sd_try!(ffi::sd_journal_seek_realtime_usec(self.j, usec))
            }
            JournalSeek::Cursor { cursor } => {
                sd_try!(ffi::sd_journal_seek_cursor(self.j, cursor.as_ptr() as *const c_char))
            }
        };
        let c: *mut c_char = ptr::null_mut();
        if unsafe { ffi::sd_journal_get_cursor(self.j, &c) != 0 } {
            // Cursor may need to be re-aligned on a real entry first.
            sd_try!(ffi::sd_journal_next(self.j));
            sd_try!(ffi::sd_journal_get_cursor(self.j, &c));
        }
        let cs = unsafe { MString::from_raw(c) };
        let cs = try!(cs.or(Err(io::Error::new(InvalidData, "invalid cursor"))));
        Ok(cs.to_string())
    }

    /// Returns the cursor of current journal entry
    pub fn cursor(&self) -> Result<String> {
        let mut c_cursor: *mut c_char = ptr::null_mut();

        sd_try!(ffi::sd_journal_get_cursor(self.j, &mut c_cursor));

        let cursor = unsafe { MString::from_raw(c_cursor) };
        let cursor = try!(cursor.or(Err(io::Error::new(InvalidData, "invalid cursor"))));
        Ok(cursor.to_string())
    }

    /// Returns timestamp at which current journal is recorded
    pub fn timestamp(&self) -> Result<time::SystemTime> {
        let mut timestamp_us: u64 = 0;
        sd_try!(ffi::sd_journal_get_realtime_usec(self.j, &mut timestamp_us));
        Ok(system_time_from_realtime_usec(timestamp_us))
    }

    /// Adds a match by which to filter the entries of the journal.
    /// If a match is applied, only entries with this field set will be iterated.
    pub fn match_add<T: Into<Vec<u8>>>(&mut self, key: &str, val: T) -> Result<&mut Journal> {
        let mut filter = Vec::<u8>::from(key);
        filter.push('=' as u8);
        filter.extend(val.into());
        let data = filter.as_ptr() as *const c_void;
        let datalen = filter.len() as size_t;
        sd_try!(ffi::sd_journal_add_match(self.j, data, datalen));
        Ok(self)
    }

    /// Inserts a disjunction (i.e. logical OR) in the match list.
    pub fn match_or(&mut self) -> Result<&mut Journal> {
        sd_try!(ffi::sd_journal_add_disjunction(self.j));
        Ok(self)
    }

    /// Inserts a conjunction (i.e. logical AND) in the match list.
    pub fn match_and(&mut self) -> Result<&mut Journal> {
        sd_try!(ffi::sd_journal_add_conjunction(self.j));
        Ok(self)
    }

    /// Flushes all matches, disjunction and conjunction terms.
    /// After this call all filtering is removed and all entries in
    /// the journal will be iterated again.
    pub fn match_flush(&mut self) -> Result<&mut Journal> {
        unsafe { ffi::sd_journal_flush_matches(self.j) };
        Ok(self)
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
