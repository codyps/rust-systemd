use libc::{c_char, c_int, size_t};
use log::{self, Log, Record, Level, SetLoggerError};
use std::{io, ptr, result, fmt};
use std::collections::BTreeMap;
use std::ffi::CString;
use std::io::ErrorKind::InvalidData;
use std::os::raw::c_void;
use std::u64;
use crate::ffi::array_to_iovecs;
use crate::ffi::id128::sd_id128_t;
use crate::ffi::journal as ffi;
use crate::id128::Id128;
use super::{free_cstring, Result, usec_from_duration};
use std::time;
use foreign_types::{foreign_type, ForeignType};

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

enum SyslogLevel {
    // Emerg = 0,
    // Alert = 1,
    // Crit = 2,
    Err = 3,
    Warning = 4,
    // Notice = 5,
    Info = 6,
    Debug = 7,
}

/// Record a log entry, with custom priority and location.
pub fn log(level: usize, file: &str, line: u32, module_path: &str, args: &fmt::Arguments) {
    send(&[&format!("PRIORITY={}", level),
           &format!("MESSAGE={}", args),
           &format!("CODE_LINE={}", line),
           &format!("CODE_FILE={}", file),
           &format!("CODE_FUNCTION={}", module_path)]);
}


/// Send a `log::Record` to systemd-journald.
pub fn log_record(record: &Record) {
    let lvl = match record.level() {
        Level::Error => SyslogLevel::Err,
        Level::Warn => SyslogLevel::Warning,
        Level::Info => SyslogLevel::Info,
        Level::Debug |
        Level::Trace => SyslogLevel::Debug,
    } as usize;

    let mut keys = vec![
        format!("PRIORITY={}", lvl),
        format!("MESSAGE={}", record.args()),
        format!("TARGET={}", record.target()),
    ];

    if let Some(line) = record.line() { keys.push(format!("CODE_LINE={}", line)) }
    if let Some(file) = record.file() { keys.push(format!("CODE_FILE={}", file)) }
    if let Some(module_path) = record.module_path() { keys.push(format!("CODE_FUNCTION={}", module_path)) }

    let str_keys = keys.iter().map(AsRef::as_ref).collect::<Vec<_>>();
    send(&str_keys);
}

/// Logger implementation over systemd-journald.
pub struct JournalLog;
impl Log for JournalLog {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        log_record(record);
    }

    fn flush(&self) {
        // There is no flushing required.
    }
}

static LOGGER: JournalLog = JournalLog;
impl JournalLog {
    pub fn init() -> result::Result<(), SetLoggerError> {
        log::set_logger(&LOGGER)
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

foreign_type! {
    /// A reader for systemd journal.
    ///
    /// Supports read, next, previous, and seek operations.
    pub unsafe type Journal {
        type CType = ffi::sd_journal;
        fn drop = ffi::sd_journal_close;
    }
}

/// Represents the set of journal files to read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum JournalFiles {
    /// The system-wide journal.
    System,
    /// The current user's journal.
    CurrentUser,
    /// All journal files, including other users'.
    All,
}

impl JournalFiles {
    fn as_flags(self) -> c_int {
        match self {
            JournalFiles::System => ffi::SD_JOURNAL_SYSTEM,
            JournalFiles::CurrentUser => ffi::SD_JOURNAL_CURRENT_USER,
            JournalFiles::All => 0,
        }
    }
}

/// Seeking position in journal.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Clone, Debug)]
pub enum JournalWaitResult {
    Nop,
    Append,
    Invalidate
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
        let mut flags = files.as_flags();
        if runtime_only {
            flags |= ffi::SD_JOURNAL_RUNTIME_ONLY;
        }
        if local_only {
            flags |= ffi::SD_JOURNAL_LOCAL_ONLY;
        }

        let mut jp = ptr::null_mut();
        sd_try!(ffi::sd_journal_open(&mut jp, flags));
        let journal = unsafe { Journal::from_ptr(jp) };
        sd_try!(ffi::sd_journal_seek_head(journal.as_ptr()));
        Ok(journal)
    }

    /// Open the systemd journal located in a specific folder for reading.
    ///
    /// Params:
    ///
    /// * path: the absolute directory path. All journal files in this directory
    ///   will be opened and interleaved automatically.
    /// * files: the set of journal files to read. If the calling process
    ///   doesn't have permission to read the system journal, a call to
    ///   `Journal::open` with `System` or `All` will succeed, but system
    ///   journal entries won't be included. This behavior is due to systemd.
    /// * os_root: if true, journal files are searched for below the usual
    ///   /var/log/journal and /run/log/journal relative to the specified path,
    ///   instead of directly beneath it.
    pub fn open_directory(path: &std::path::Path, files: JournalFiles, os_root: bool) -> Result<Journal> {
        let c_path = CString::new(path.to_str().unwrap()).unwrap();
        let mut flags: c_int = 0;
        if os_root {
            flags |= ffi::SD_JOURNAL_OS_ROOT;
        }
        flags |= match files {
            JournalFiles::System => ffi::SD_JOURNAL_SYSTEM,
            JournalFiles::CurrentUser => ffi::SD_JOURNAL_CURRENT_USER,
            JournalFiles::All => 0
        };

        let mut jp = ptr::null_mut();
        sd_try!(ffi::sd_journal_open_directory(&mut jp, c_path.as_ptr(), flags));
        let journal = unsafe { Journal::from_ptr(jp) };
        sd_try!(ffi::sd_journal_seek_head(journal.as_ptr()));
        Ok(journal)
    }

    /// Open the systemd journal located in a specific folder for reading.
    ///
    /// Params:
    ///
    /// * path: the absolute directory path. All journal files in this directory
    ///   will be opened and interleaved automatically.
    pub fn open_files(paths: &[&std::path::Path]) -> Result<Journal> {
        let mut c_paths: Vec<std::rc::Rc<CString>> = std::vec::Vec::new();
        let mut c_paths_ptr: Vec<*const c_char> = std::vec::Vec::new();
        for path in paths {
            let c_path = std::rc::Rc::new(CString::new(path.to_str().unwrap()).unwrap());
            c_paths.push(c_path.clone());
            c_paths_ptr.push(c_path.as_ptr());
        }
        c_paths_ptr.push(ptr::null_mut());
        // let c_path = CString::new(path.to_str().unwrap()).unwrap();
        let mut c_flags: c_int = 0;
        let mut jp = ptr::null_mut();
        sd_try!(ffi::sd_journal_open_files(&mut jp, c_paths_ptr.as_ptr(), c_flags));
        let journal = unsafe { Journal::from_ptr(jp) };
        sd_try!(ffi::sd_journal_seek_head(journal.as_ptr()));
        Ok(journal)
    }

    /// Get and parse the currently journal record from the journal
    /// It returns Result<Option<...>> out of convenience for calling
    /// functions. It always returns Ok(Some(...)) if successful.
    fn get_record(&mut self) -> Result<Option<JournalRecord>> {
        unsafe { ffi::sd_journal_restart_data(self.as_ptr()) }

        let mut ret: JournalRecord = BTreeMap::new();

        let mut sz: size_t = 0;
        let mut data: *const u8 = ptr::null_mut();
        while sd_try!(ffi::sd_journal_enumerate_data(self.as_ptr(), &mut data, &mut sz)) > 0 {
            unsafe {
                let b = std::slice::from_raw_parts(data, sz as usize);
                let field = String::from_utf8_lossy(b);
                let mut name_value = field.splitn(2, '=');
                let name = name_value.next().unwrap();
                let value = name_value.next().unwrap();
                ret.insert(From::from(name), From::from(value));
            }
        }

        Ok(Some(ret))
    }

    /// Read the next record from the journal. Returns `Ok(None)` if there
    /// are no more records to read.
    pub fn next_record(&mut self) -> Result<Option<JournalRecord>> {
        if sd_try!(ffi::sd_journal_next(self.as_ptr())) == 0 {
            return Ok(None);
        }

        self.get_record()
        
    }

    /// Read the previous record from the journal. Returns `Ok(None)` if there
    /// are no more records to read.
    pub fn previous_record(&mut self) -> Result<Option<JournalRecord>> {
        if sd_try!(ffi::sd_journal_previous(self.as_ptr())) == 0 {
            return Ok(None);
        }
        
        self.get_record()
    }

    /// Wait for next record to arrive.
    /// Pass wait_time `None` to wait for an unlimited period for new records.
    fn wait(&mut self, wait_time: Option<time::Duration>) -> Result<JournalWaitResult> {

        let time = wait_time.map(usec_from_duration).unwrap_or(::std::u64::MAX);

        match sd_try!(ffi::sd_journal_wait(self.as_ptr(), time)) {
            ffi::SD_JOURNAL_NOP => Ok(JournalWaitResult::Nop),
            ffi::SD_JOURNAL_APPEND => Ok(JournalWaitResult::Append),
            ffi::SD_JOURNAL_INVALIDATE => Ok(JournalWaitResult::Invalidate),
            _ => Err(io::Error::new(InvalidData, "Failed to wait for changes"))
        }
    }

    /// Wait for the next record to appear. Returns `Ok(None)` if there were no
    /// new records in the given wait time.
    /// Pass wait_time `None` to wait for an unlimited period for new records.
    pub fn await_next_record(&mut self, wait_time: Option<time::Duration>) -> Result<Option<JournalRecord>> {
        match self.wait(wait_time)? {
            JournalWaitResult::Nop => Ok(None),
            JournalWaitResult::Append => self.next_record(),

            // This is possibly wrong, but I can't generate a scenario with 
            // ..::Invalidate and neither systemd's journalctl, 
            // systemd-journal-upload, and other utilities handle that case.
            JournalWaitResult::Invalidate => self.next_record()
        }
    }

    /// Iterate through all elements from the current cursor, then await the
    /// next record(s) and wait again.
    pub fn watch_all_elements<F>(&mut self, mut f: F) -> Result<()>
        where F: FnMut(JournalRecord) -> Result<()> {
            loop {
                let candidate = self.next_record()?;
                let rec = match candidate {
                    Some(rec) => rec,
                    None => { loop {
                        if let Some(r) = self.await_next_record(None)? {
                            break r;
                        }
                    }}
                };
                f(rec)?
            }
        }

    /// Seek to a specific position in journal. On success, returns a cursor
    /// to the current entry.
    pub fn seek(&mut self, seek: JournalSeek) -> Result<String> {
        let mut tail = false;
        match seek {
            JournalSeek::Head => sd_try!(ffi::sd_journal_seek_head(self.as_ptr())),
            JournalSeek::Current => 0,
            JournalSeek::Tail => {
                tail = true;
                sd_try!(ffi::sd_journal_seek_tail(self.as_ptr()))
            }
            JournalSeek::ClockMonotonic { boot_id, usec } => {
                sd_try!(ffi::sd_journal_seek_monotonic_usec(self.as_ptr(),
                                                            sd_id128_t {
                                                                bytes: *boot_id.as_bytes(),
                                                            },
                                                            usec))
            }
            JournalSeek::ClockRealtime { usec } => {
                sd_try!(ffi::sd_journal_seek_realtime_usec(self.as_ptr(), usec))
            }
            JournalSeek::Cursor { cursor } => {
                let c = CString::new(cursor)?;
                sd_try!(ffi::sd_journal_seek_cursor(self.as_ptr(), c.as_ptr()))
            }
        };
        let mut c: *const c_char = ptr::null_mut();
        if unsafe { ffi::sd_journal_get_cursor(self.as_ptr(), &mut c) != 0 } {
            // Cursor may need to be re-aligned on a real entry first.
            if tail {
                sd_try!(ffi::sd_journal_previous(self.as_ptr()));
            } else {
                sd_try!(ffi::sd_journal_next(self.as_ptr()));
            }
            sd_try!(ffi::sd_journal_get_cursor(self.as_ptr(), &mut c));
        }
        let cs = unsafe { free_cstring(c as *mut _).unwrap() };
        Ok(cs)
    }

    /// Returns the cursor of current journal entry.
    pub fn cursor(&self) -> Result<String> {
        let mut c_cursor: *const c_char = ptr::null_mut();

        sd_try!(ffi::sd_journal_get_cursor(self.as_ptr(), &mut c_cursor));
        let cursor = unsafe { free_cstring(c_cursor as *mut _).unwrap() };
        Ok(cursor)
    }

    /// Returns timestamp at which current journal entry is recorded.
    pub fn timestamp(&self) -> Result<time::SystemTime> {
        let mut timestamp_us: u64 = 0;
        sd_try!(ffi::sd_journal_get_realtime_usec(self.as_ptr(), &mut timestamp_us));
        Ok(system_time_from_realtime_usec(timestamp_us))
    }

    /// Returns monotonic timestamp and boot ID at which current journal entry is recorded.
    pub fn monotonic_timestamp(&self) -> Result<(u64, Id128)> {
        let mut monotonic_timestamp_us: u64 = 0;
        let mut id = Id128::default();
        sd_try!(ffi::sd_journal_get_monotonic_usec(
            self.as_ptr(),
            &mut monotonic_timestamp_us,
            &mut id.inner,
        ));
        Ok((monotonic_timestamp_us, id))
    }

    /// Returns monotonic timestamp at which current journal entry is recorded. Returns an error if
    /// the current entry is not from the current system boot.
    pub fn monotonic_timestamp_current_boot(&self) -> Result<u64> {
        let mut monotonic_timestamp_us: u64 = 0;
        sd_try!(ffi::sd_journal_get_monotonic_usec(
            self.as_ptr(),
            &mut monotonic_timestamp_us,
            ptr::null_mut(),
        ));
        Ok(monotonic_timestamp_us)
    }

    /// Adds a match by which to filter the entries of the journal.
    /// If a match is applied, only entries with this field set will be iterated.
    pub fn match_add<T: Into<Vec<u8>>>(&mut self, key: &str, val: T) -> Result<&mut Journal> {
        let mut filter = Vec::<u8>::from(key);
        filter.push(b'=');
        filter.extend(val.into());
        let data = filter.as_ptr() as *const c_void;
        let datalen = filter.len() as size_t;
        sd_try!(ffi::sd_journal_add_match(self.as_ptr(), data, datalen));
        Ok(self)
    }

    /// Inserts a disjunction (i.e. logical OR) in the match list.
    pub fn match_or(&mut self) -> Result<&mut Journal> {
        sd_try!(ffi::sd_journal_add_disjunction(self.as_ptr()));
        Ok(self)
    }

    /// Inserts a conjunction (i.e. logical AND) in the match list.
    pub fn match_and(&mut self) -> Result<&mut Journal> {
        sd_try!(ffi::sd_journal_add_conjunction(self.as_ptr()));
        Ok(self)
    }

    /// Flushes all matches, disjunction and conjunction terms.
    /// After this call all filtering is removed and all entries in
    /// the journal will be iterated again.
    pub fn match_flush(&mut self) -> Result<&mut Journal> {
        unsafe { ffi::sd_journal_flush_matches(self.as_ptr()) };
        Ok(self)
    }
}
