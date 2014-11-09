use libc::{c_int,size_t};
use log::{Logger,LogRecord,LogLevel,LogLocation};
use std::{fmt,io,ptr};
use std::collections::BTreeMap;
use ffi;

/// Send preformatted fields to systemd.
///
/// This is a relatively low-level operation and probably not suitable unless
/// you need precise control over which fields are sent to systemd.
pub fn send(args : &[&str]) -> c_int {
    let iovecs = ffi::array_to_iovecs(args);
    unsafe { ffi::sd_journal_sendv(iovecs.as_ptr(), iovecs.len() as c_int) }
}

/// Send a simple message to systemd.
pub fn print(lvl : uint, s : &str) -> c_int {
    send([
         format!("PRIORITY={}", lvl).as_slice(),
         format!("MESSAGE={}", s).as_slice()
         ])
}

/// Send a `log::LogRecord` to systemd.
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

#[experimental]
pub type JournalRecord = BTreeMap<String, String>;

/// A cursor into the systemd journal.
///
/// Supports read, next, previous, and seek operations.
pub struct Journal {
    j: ffi::sd_journal
}

/// Represents the set of journal files to read.
#[stable]
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
    pub fn open(files: JournalFiles, runtime_only: bool, local_only: bool) -> io::IoResult<Journal> {
        let mut flags: c_int = 0;
        if runtime_only {
            flags |= ffi::SD_JOURNAL_RUNTIME_ONLY;
        }
        if local_only {
            flags |= ffi::SD_JOURNAL_LOCAL_ONLY;
        }
        flags |= match files {
            System => ffi::SD_JOURNAL_SYSTEM,
            CurrentUser => ffi::SD_JOURNAL_CURRENT_USER,
            All => 0
        };

        let journal = Journal { j: ptr::null_mut() };
        sd_try!(ffi::sd_journal_open(&journal.j, flags));
        sd_try!(ffi::sd_journal_seek_head(journal.j));
        Ok(journal)
    }

    /// Read the next record from the journal. Returns `io::EndOfFile` if there
    /// are no more records to read.
    pub fn next_record(&self) -> io::IoResult<JournalRecord> {
        if sd_try!(ffi::sd_journal_next(self.j)) == 0 {
            return Err(io::IoError {
                kind: io::EndOfFile,
                desc: "end of journal",
                detail: None
            });
        }
        unsafe { ffi::sd_journal_restart_data(self.j) }

        let mut ret: JournalRecord = BTreeMap::new();
        
        let mut sz: size_t = 0;
        let data: *mut u8 = ptr::null_mut();
        while sd_try!(ffi::sd_journal_enumerate_data(self.j, &data, &mut sz)) > 0 {
            unsafe {
                ::collections::slice::raw::mut_buf_as_slice(data, sz as uint, |b| {
                    let field = ::std::str::raw::from_utf8(b);
                    let name_value = field.splitn(1, '=');
                    ret.insert(
                        String::from_str(name_value.next()),
                        String::from_str(name_value.next()));
                });
            }
        }

        Ok(ret)
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

