use super::{free_cstring, usec_from_duration, Result};
use crate::ffi::journal as ffi;
use crate::ffi::ConstIovec;
use crate::id128::Id128;
use cstr_argument::CStrArgument;
use foreign_types::{foreign_type, ForeignType, ForeignTypeRef};
use libc::{c_char, c_int, size_t};
use log::{self, Level, Log, Record, SetLoggerError};
use memchr::memchr;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::io::ErrorKind::InvalidData;
use std::mem::MaybeUninit;
use std::os::raw::c_void;
use std::os::unix::io::AsRawFd;
use std::{fmt, io, ptr, result, slice, time};

fn collect_and_send<T, S>(args: T) -> c_int
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    let iovecs: Vec<ConstIovec> = args
        // SAFETY: we manually guarantee that the lifetime of ConstIovec does not exceed that of
        // the data it's referencing in order to avoid additional allocations.
        .map(|x| unsafe { ConstIovec::from_str(x) })
        .collect();
    unsafe { ffi::sd_journal_sendv(iovecs.as_ptr(), iovecs.len() as c_int) }
}

/// Send preformatted fields to systemd.
///
/// This is a relatively low-level operation and probably not suitable unless
/// you need precise control over which fields are sent to systemd.
pub fn send(args: &[&str]) -> c_int {
    collect_and_send(args.iter())
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
    Notice = 5,
    Info = 6,
    Debug = 7,
}

impl From<log::Level> for SyslogLevel {
    fn from(level: log::Level) -> Self {
        match level {
            Level::Error => SyslogLevel::Err,
            Level::Warn => SyslogLevel::Warning,
            Level::Info => SyslogLevel::Notice,
            Level::Debug => SyslogLevel::Info,
            Level::Trace => SyslogLevel::Debug,
        }
    }
}

/// Record a log entry, with custom priority and location.
pub fn log(level: usize, file: &str, line: u32, module_path: &str, args: &fmt::Arguments<'_>) {
    send(&[
        &format!("PRIORITY={}", level),
        &format!("MESSAGE={}", args),
        &format!("CODE_LINE={}", line),
        &format!("CODE_FILE={}", file),
        &format!("CODE_MODULE={}", module_path),
    ]);
}

/// Send a `log::Record` to systemd-journald.
pub fn log_record(record: &Record<'_>) {
    let keys = [
        format!("PRIORITY={}", SyslogLevel::from(record.level()) as usize),
        format!("MESSAGE={}", record.args()),
        format!("TARGET={}", record.target()),
    ];
    let opt_keys = [
        record.line().map(|line| format!("CODE_LINE={}", line)),
        record.file().map(|file| format!("CODE_FILE={}", file)),
        record
            .module_path()
            .map(|path| format!("CODE_FUNC={}", path)),
    ];

    collect_and_send(keys.iter().chain(opt_keys.iter().flatten()));
}

/// Logger implementation over systemd-journald.
pub struct JournalLog;
impl Log for JournalLog {
    fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &Record<'_>) {
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

foreign_type! {
    /// A reader for systemd journal.
    ///
    /// Supports read, next, previous, and seek operations.
    ///
    /// Note that the `Journal` is not `Send` nor `Sync`: it cannot be used in any thread other
    /// than the one which creates it.
    pub unsafe type Journal {
        type CType = ffi::sd_journal;
        fn drop = ffi::sd_journal_close;
    }
}

/// A (name, value) pair formatted as a "NAME=value" byte string
///
/// Internally, each journal entry includes a variety of these data entries.
#[derive(Debug, PartialEq, Eq)]
pub struct JournalEntryField<'a> {
    // TODO: this could be a CStr, which might be useful for downstream consumers
    data: &'a [u8],
    eq_offs: usize,
}

impl<'a> JournalEntryField<'a> {
    /// The entire data element
    pub fn data(&self) -> &[u8] {
        self.data
    }

    /// The name (part before the `=`). The `=` is not included
    ///
    /// Note that depending on how this is retrieved, it might be truncated (ie: incomplete), see
    /// `set_data_threshold()` for details.
    pub fn name(&self) -> &[u8] {
        &self.data[..self.eq_offs]
    }

    /// The value, part after the `=`, if present. The `=` is not included.
    ///
    /// Note that depending on how this is retrieved, it might be truncated (ie: incomplete), see
    /// `set_data_threshold()` for details.
    pub fn value(&self) -> Option<&[u8]> {
        if self.eq_offs != self.data.len() {
            Some(&self.data[(self.eq_offs + 1)..])
        } else {
            None
        }
    }
}

impl<'a> From<&'a [u8]> for JournalEntryField<'a> {
    fn from(data: &'a [u8]) -> Self {
        // find the `=`
        let eq_offs = match memchr(b'=', data) {
            Some(v) => v,
            None => data.len(),
        };

        Self { data, eq_offs }
    }
}

/*
impl Iterator for JournalEntry<'a> {
    type Item = Result<JournalEntryEntry<'a>>;

    pub fn next(&mut self) -> Option<Self::Item> {
        let r = crate::ffi_result(unsafe { ffi::sd_journal_enumerate_data(
            self.as_ptr(),
            &mut data,
            &mut sz)});

        let v = match r {
            Err(e) => return Some(Err(e)),
            Ok(v) => v,
        };

        if v == 0 {
            return None;
        }

        // WARNING: slice is only valid until next call to one of `sd_journal_enumerate_data`,
        // `sd_journal_get_data`, or `sd_journal_enumerate_avaliable_data`.
        let b = unsafe { std::slice::from_raw_parts(data, sz as usize) };
        let field = String::from_utf8_lossy(b);
        let mut name_value = field.splitn(2, '=');
        let name = name_value.next().unwrap();
        let value = name_value.next().unwrap();
        }
    }
}
*/

// A single log entry from journal.
pub type JournalRecord = BTreeMap<String, String>;

/// Represents the set of journal files to read.
#[deprecated(
    since = "0.8.0",
    note = "Use `OpenOptions` instead. `JournalFiles` doesn't completely represent the filtering/inclusion options"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum JournalFiles {
    /// The system-wide journal.
    System,
    /// The current user's journal.
    CurrentUser,
    /// All journal files, including other users'.
    All,
}

#[allow(deprecated)]
impl JournalFiles {
    fn as_flags(self) -> c_int {
        match self {
            JournalFiles::System => ffi::SD_JOURNAL_SYSTEM,
            JournalFiles::CurrentUser => ffi::SD_JOURNAL_CURRENT_USER,
            JournalFiles::All => 0,
        }
    }
}

/// A wrapper type that allows displaying a single entry in the journal
pub struct DisplayEntryData<'a> {
    // RULES:
    //  - we can't move the cursor/position in the journal (no seeking, no
    //   iteration/next/previous)
    //  - we have _total_ ownership over data iteration. Do what ever necessary to get all the data
    //    we want
    journal: RefCell<&'a mut JournalRef>,
}

impl<'a> fmt::Display for DisplayEntryData<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(fmt, "{{")?;

        let mut j = self.journal.borrow_mut();
        j.restart_data();
        loop {
            match j.enumerate_data() {
                Ok(Some(v)) => {
                    writeln!(fmt, " \"{}\",", std::str::from_utf8(v.data()).unwrap())?;
                }
                Ok(None) => break,
                Err(e) => {
                    writeln!(fmt, "E: {:?}", e)?;
                    break;
                }
            }
        }

        writeln!(fmt, "}}")
    }
}

impl<'a> From<&'a mut JournalRef> for DisplayEntryData<'a> {
    fn from(v: &'a mut JournalRef) -> Self {
        Self {
            journal: RefCell::new(v),
        }
    }
}

/// Seeking position in journal.
///
/// Note: variants corresponding to [`Journal::next_skip()`] and [`Journal::previous_skip()`] are
/// omitted because those are treated by sd-journal as pieces of journal iteration in that when
/// they complete the journal is at a specific entry. All the seek type operations don't behave as
/// part of iteration, and don't place the journal at a specific entry (iteration must be used to
/// move to a journal entry).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum JournalSeek {
    Head,
    Tail,
    ClockMonotonic { boot_id: Id128, usec: u64 },
    ClockRealtime { usec: u64 },
    Cursor { cursor: String },
}

#[derive(Clone, Debug)]
pub enum JournalWaitResult {
    Nop,
    Append,
    Invalidate,
}

/// Open a [`Journal`], using custom options.
///
/// This corresponds to [`sd_journal_open_namespace()`] and [`sd_journal_open()`].
///
/// `sd_journal_open()`: https://www.freedesktop.org/software/systemd/man/sd_journal_open.html
/// `sd_journal_open_namespace()`: https://www.freedesktop.org/software/systemd/man/sd_journal_open.html
///
/// # Examples
///
/// ```
/// use systemd::{Journal, journal};
/// let _ : Journal = journal::OpenOptions::default()
///     .current_user(true)
///     .open().unwrap();
/// ```
#[derive(Clone, Debug, Default)]
pub struct OpenOptions {
    current_user: bool,
    system: bool,
    local_only: bool,
    runtime_only: bool,
    all_namespaces: bool,
    include_default_namespace: bool,
    extra_raw_flags: libc::c_int,
}

impl OpenOptions {
    /// Open the journal files of the current user.
    ///
    /// If neither this nor `system` is `true`, all files will be opened.
    ///
    /// This corresponds to `SD_JOURNAL_CURRENT_USER`
    pub fn current_user(&mut self, current_user: bool) -> &mut Self {
        self.current_user = current_user;
        self
    }

    /// Open the journal files of system services and the kernel
    ///
    /// If neither this nor `current_user` is `true`, all files will be opened
    ///
    /// This corresponds to `SD_JOURNAL_SYSTEM`
    pub fn system(&mut self, system: bool) -> &mut Self {
        self.system = system;
        self
    }

    /// Only journal files generated on the local machine will be opened
    ///
    /// This corresponds to `SD_JOURNAL_LOCAL_ONLY`
    pub fn local_only(&mut self, local_only: bool) -> &mut Self {
        self.local_only = local_only;
        self
    }

    /// Only volatile journal files will be opened, excluding those stored on persistent storage
    ///
    /// This corresponds to `SD_JOURNAL_RUNTIME_ONLY`
    pub fn runtime_only(&mut self, runtime_only: bool) -> &mut Self {
        self.runtime_only = runtime_only;
        self
    }

    /// Access all defined namespaces simultaneously (`namespace` is ignored if this is `true`)
    ///
    /// This corresponds to `SD_JOURNAL_ALL_NAMESPACES`
    pub fn all_namespaces(&mut self, all_namespaces: bool) -> &mut Self {
        self.all_namespaces = all_namespaces;
        self
    }

    /// The specified `namespace` and the default namespace are accessed but no others
    ///
    /// If `namespace` is not provided, this has no effect.
    ///
    /// This corresponds to `SD_JOURNAL_INCLUDE_DEFAULT_NAMESPACE`
    pub fn include_default_namespace(&mut self, include_default_namespace: bool) -> &mut Self {
        self.include_default_namespace = include_default_namespace;
        self
    }

    /// Supply any additional flags to the `open*()` function
    pub fn extra_raw_flags(&mut self, extra_raw_flags: libc::c_int) -> &mut Self {
        self.extra_raw_flags = extra_raw_flags;
        self
    }

    /// Open the log journal for reading. Entries included are dependent on options.
    ///
    /// This corresponds to [`sd_journal_open()`]
    ///
    /// `sd_journal_open()`: https://www.freedesktop.org/software/systemd/man/sd_journal_open.html
    pub fn open(&self) -> Result<Journal> {
        Journal::open_with_opts::<&std::ffi::CStr>(self)
    }

    /// Open the log journal for reading in the given namespace. Entries included are dependent on
    /// options.
    ///
    /// Note that some options (`SD_JOURNAL_ALL_NAMESPACES`) affect whether `namespace` is
    /// considered. Our API doesn't check for unused data here, but users are encouraged to avoid
    /// passing unused data by using [`OpenOptions::open()`] instead when a namespace argument is
    /// not required.
    ///
    /// This corresponds to [`sd_journal_open_namespace()`]
    ///
    /// `sd_journal_open_namespace()`: https://www.freedesktop.org/software/systemd/man/sd_journal_open.html
    #[cfg(feature = "systemd_v245")]
    #[cfg_attr(docsrs, doc(cfg(feature = "systemd_v245")))]
    pub fn open_namespace<A: CStrArgument>(&self, namespace: A) -> Result<Journal> {
        Journal::open_with_opts_ns(Some(namespace), self)
    }
}

/// Open a journal, specifying a directory
#[derive(Clone, Debug, Default)]
pub struct OpenDirectoryOptions {
    os_root: bool,
    current_user: bool,
    system: bool,
    extra_raw_flags: libc::c_int,
}

impl OpenDirectoryOptions {
    /// If true, journal files are searched for below the usual `/var/log/journal` and
    /// `/run/log/journal` relative to the specified directory instead of directly beneath it
    pub fn os_root(&mut self, os_root: bool) -> &mut Self {
        self.os_root = os_root;
        self
    }

    /// Open the journal files of the current user.
    ///
    /// If neither this nor `system` is `true`, all files will be opened.
    ///
    /// This corresponds to `SD_JOURNAL_CURRENT_USER`
    pub fn current_user(&mut self, current_user: bool) -> &mut Self {
        self.current_user = current_user;
        self
    }

    /// Open the journal files of system services and the kernel
    ///
    /// If neither this nor `current_user` is `true`, all files will be opened
    ///
    /// This corresponds to `SD_JOURNAL_SYSTEM`
    pub fn system(&mut self, system: bool) -> &mut Self {
        self.system = system;
        self
    }

    /// Supply any additional flags to the `open*()` function
    pub fn extra_raw_flags(&mut self, extra_raw_flags: libc::c_int) -> &mut Self {
        self.extra_raw_flags = extra_raw_flags;
        self
    }

    /// Open the journal corresponding to this directory path
    pub fn open_directory<A: CStrArgument>(&self, directory: A) -> Result<Journal> {
        Journal::open_with_opts_dir(directory, self)
    }

    /*
    unsafe pub fn open_directory_fd<A: AsRawFd>(&self, directory: A) -> Result<Journal> {
        todo!()
    }
    */
}

/// Open a journal, specifying one or more files
///
/// Note that when used on a live journal, files may be rotated at any time and as a result the
/// opening of specific files is inherently racy.
#[derive(Clone, Debug, Default)]
pub struct OpenFilesOptions {
    extra_raw_flags: libc::c_int,
}

impl OpenFilesOptions {
    /// Supply any additional flags to the `open*()` function
    ///
    /// Note that as of writing, no flags are accepted by the underlying function in sd-journal
    pub fn extra_raw_flags(&mut self, extra_raw_flags: libc::c_int) -> &mut Self {
        self.extra_raw_flags = extra_raw_flags;
        self
    }

    /// Open a journal, giving one or more paths to files
    pub fn open_files<A: CStrArgument, I: IntoIterator<Item = A>>(
        &self,
        files: I,
    ) -> Result<Journal> {
        Journal::open_with_opts_files(files, self)
    }

    /*
    /// Open a journal, giving one or more file descriptors referring to open files
    unsafe pub fn open_files_fd<A: AsRawFd, I: IntoIterator<Item = A>> (
        &self,
        files: I,
    ) -> Result<Journal> {
        todo!()
    }
    */
}

impl Journal {
    fn open_with_opts<A: CStrArgument>(opts: &OpenOptions) -> Result<Journal> {
        let mut flags = opts.extra_raw_flags;
        if opts.current_user {
            flags |= ffi::SD_JOURNAL_CURRENT_USER;
        }
        if opts.system {
            flags |= ffi::SD_JOURNAL_SYSTEM;
        }
        if opts.local_only {
            flags |= ffi::SD_JOURNAL_LOCAL_ONLY;
        }
        if opts.local_only {
            flags |= ffi::SD_JOURNAL_LOCAL_ONLY;
        }
        if opts.runtime_only {
            flags |= ffi::SD_JOURNAL_RUNTIME_ONLY;
        }
        if opts.all_namespaces {
            flags |= ffi::SD_JOURNAL_ALL_NAMESPACES;
        }
        if opts.include_default_namespace {
            flags |= ffi::SD_JOURNAL_INCLUDE_DEFAULT_NAMESPACE;
        }

        let mut jp = MaybeUninit::uninit();
        crate::ffi_result(unsafe { ffi::sd_journal_open(jp.as_mut_ptr(), flags) })?;
        Ok(unsafe { Journal::from_ptr(jp.assume_init()) })
    }

    #[cfg(feature = "systemd_v245")]
    fn open_with_opts_ns<A: CStrArgument>(
        namespace: Option<A>,
        opts: &OpenOptions,
    ) -> Result<Journal> {
        let mut flags = opts.extra_raw_flags;
        if opts.current_user {
            flags |= ffi::SD_JOURNAL_CURRENT_USER;
        }
        if opts.system {
            flags |= ffi::SD_JOURNAL_SYSTEM;
        }
        if opts.local_only {
            flags |= ffi::SD_JOURNAL_LOCAL_ONLY;
        }
        if opts.local_only {
            flags |= ffi::SD_JOURNAL_LOCAL_ONLY;
        }
        if opts.runtime_only {
            flags |= ffi::SD_JOURNAL_RUNTIME_ONLY;
        }
        if opts.all_namespaces {
            flags |= ffi::SD_JOURNAL_ALL_NAMESPACES;
        }
        if opts.include_default_namespace {
            flags |= ffi::SD_JOURNAL_INCLUDE_DEFAULT_NAMESPACE;
        }

        let ns = namespace.map(|a| a.into_cstr());
        let ns_p = ns
            .as_ref()
            .map(|a| a.as_ref().as_ptr())
            .unwrap_or(ptr::null());
        let mut jp = MaybeUninit::uninit();
        crate::ffi_result(unsafe { ffi::sd_journal_open_namespace(jp.as_mut_ptr(), ns_p, flags) })?;
        Ok(unsafe { Journal::from_ptr(jp.assume_init()) })
    }

    fn open_with_opts_dir<A: CStrArgument>(
        directory: A,
        opts: &OpenDirectoryOptions,
    ) -> Result<Journal> {
        let mut flags = opts.extra_raw_flags;
        if opts.os_root {
            flags |= ffi::SD_JOURNAL_OS_ROOT;
        }
        if opts.current_user {
            flags |= ffi::SD_JOURNAL_CURRENT_USER;
        }
        if opts.system {
            flags |= ffi::SD_JOURNAL_SYSTEM;
        }

        let d_path = directory.into_cstr();
        let mut jp = MaybeUninit::uninit();
        crate::ffi_result(unsafe {
            ffi::sd_journal_open_directory(jp.as_mut_ptr(), d_path.as_ref().as_ptr(), flags)
        })?;
        Ok(unsafe { Journal::from_ptr(jp.assume_init()) })
    }

    fn open_with_opts_files<A: CStrArgument, I: IntoIterator<Item = A>>(
        files: I,
        opts: &OpenFilesOptions,
    ) -> Result<Journal> {
        let mut file_cstrs = Vec::new();
        for i in files {
            file_cstrs.push(i.into_cstr());
        }

        let mut file_ptrs = Vec::new();
        for i in &file_cstrs {
            file_ptrs.push(i.as_ref().as_ptr());
        }

        file_ptrs.push(ptr::null());

        let mut jp = MaybeUninit::uninit();
        crate::ffi_result(unsafe {
            ffi::sd_journal_open_files(jp.as_mut_ptr(), file_ptrs.as_ptr(), opts.extra_raw_flags)
        })?;

        Ok(unsafe { Journal::from_ptr(jp.assume_init()) })
    }

    /// Open a `Journal` corresponding to `files` for reading
    ///
    /// If the calling process doesn't have permission to read the system journal, a call to
    /// `Journal::open` with `System` or `All` will succeed, but system journal entries won't be
    /// included. This behavior is due to systemd.
    ///
    /// If `runtime_only` is true, include only journal entries from the current boot. If false,
    /// include all entries.
    ///
    /// If `local_only` is true, include only journal entries originating from localhost. If false,
    /// include all entries.
    #[deprecated(
        since = "0.8.0",
        note = "Use `OpenOptions` instead. It removes the blind boolean options, and allows complete specification of the selected/filtered files"
    )]
    #[allow(deprecated)]
    pub fn open(files: JournalFiles, runtime_only: bool, local_only: bool) -> Result<Journal> {
        OpenOptions::default()
            .extra_raw_flags(files.as_flags())
            .local_only(local_only)
            .runtime_only(runtime_only)
            .open()
    }
}

impl JournalRef {
    /// Returns a file descriptor  a file descriptor that may be
    /// asynchronously polled in an external event loop and is signaled as
    /// soon as the journal changes, because new entries or files were added,
    /// rotation took place, or files have been deleted, and similar. The
    /// file descriptor is suitable for usage in poll(2).
    ///
    /// This corresponds to [`sd_journal_get_fd`]
    ///
    /// [`sd_journal_get_fd`]: https://www.freedesktop.org/software/systemd/man/sd_journal_get_fd.html
    #[inline]
    pub fn fd(&self) -> Result<c_int> {
        Ok(sd_try!(ffi::sd_journal_get_fd(self.as_ptr())))
    }

    /// Fields that are longer that this number of bytes _may_ be truncated when retrieved by this [`Journal`]
    /// instance.
    ///
    /// Use [`set_data_threshold()`] to adjust.
    pub fn data_threshold(&mut self) -> Result<usize> {
        let mut curr_thresh = MaybeUninit::uninit();
        crate::ffi_result(unsafe {
            ffi::sd_journal_get_data_threshold(self.as_ptr(), curr_thresh.as_mut_ptr())
        })?;

        Ok(unsafe { curr_thresh.assume_init() })
    }

    /// Set the number of bytes after which returned fields _may_ be truncated when retrieved by
    /// this [`Journal`] instance.
    ///
    /// Setting this as small as possible for your application can allow the library to avoid
    /// decompressing large objects in full.
    pub fn set_data_threshold(&mut self, new_theshold: usize) -> Result<()> {
        crate::ffi_result(unsafe {
            ffi::sd_journal_set_data_threshold(self.as_ptr(), new_theshold)
        })?;

        Ok(())
    }

    /// Get the data associated with a particular field from the current journal entry
    ///
    /// Note that this may be affected by the current data threshold, see `data_threshold()` and
    /// `set_data_threshold()`.
    ///
    /// Note: the use of `&mut` here is because calls to some (though not all) other journal
    /// functions can invalidate the reference returned within [`JournalEntryField`]. In particular:
    /// any other obtaining of data (enumerate, etc) or any adjustment of the read pointer
    /// (seeking, etc) invalidates the returned reference.
    ///
    /// Corresponds to `sd_journal_get_data()`.
    pub fn get_data<A: CStrArgument>(&mut self, field: A) -> Result<Option<JournalEntryField<'_>>> {
        let mut data = MaybeUninit::uninit();
        let mut data_len = MaybeUninit::uninit();
        let f = field.into_cstr();
        match crate::ffi_result(unsafe {
            ffi::sd_journal_get_data(
                self.as_ptr(),
                f.as_ref().as_ptr(),
                data.as_mut_ptr(),
                data_len.as_mut_ptr(),
            )
        }) {
            Ok(_) => Ok(Some(
                unsafe { slice::from_raw_parts(data.assume_init(), data_len.assume_init()) }.into(),
            )),
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Restart the iteration done by [`enumerate_data()`] and [`enumerate_avaliable_data()`] over
    /// fields of the current entry.
    ///
    /// Corresponds to `sd_journal_restart_data()`
    pub fn restart_data(&mut self) {
        unsafe { ffi::sd_journal_restart_data(self.as_ptr()) }
    }

    /// Obtain the next data
    ///
    /// Corresponds to `sd_journal_enumerate_data()`
    pub fn enumerate_data(&mut self) -> Result<Option<JournalEntryField<'_>>> {
        let mut data = MaybeUninit::uninit();
        let mut data_len = MaybeUninit::uninit();
        let r = crate::ffi_result(unsafe {
            ffi::sd_journal_enumerate_data(self.as_ptr(), data.as_mut_ptr(), data_len.as_mut_ptr())
        });

        let v = match r {
            Err(e) => return Err(e),
            Ok(v) => v,
        };

        if v == 0 {
            return Ok(None);
        }

        // WARNING: slice is only valid until next call to one of `sd_journal_enumerate_data`,
        // `sd_journal_get_data`, or `sd_journal_enumerate_avaliable_data`. This invariant is
        // maintained by our use of `&mut` above.
        let b = unsafe { std::slice::from_raw_parts(data.assume_init(), data_len.assume_init()) };
        Ok(Some(b.into()))
    }

    /// Obtain a display-able that display's the current entrie's fields
    pub fn display_entry_data(&mut self) -> DisplayEntryData<'_> {
        self.into()
    }

    /// Collect all fields of the current journal entry into a map
    ///
    /// A convenience wrapper around [`enumerate_data()`] and [`restart_data()`].
    ///
    /// This allocates/copies a lot of data. Consider using [`enumerate_data()`], etc, directly if
    /// your use case doesn't require obtaining a copy of all fields.
    fn collect_entry(&mut self) -> Result<JournalRecord> {
        let mut ret: JournalRecord = BTreeMap::new();

        self.restart_data();

        while let Some(d) = self.enumerate_data()? {
            ret.insert(
                String::from_utf8_lossy(d.name()).into(),
                String::from_utf8_lossy(d.value().unwrap()).into(),
            );
        }

        Ok(ret)
    }

    /// Iterate over journal entries.
    ///
    /// Corresponds to `sd_journal_next()`
    // TODO: consider renaming
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<u64> {
        crate::ffi_result(unsafe { ffi::sd_journal_next(self.as_ptr()) })
            .map(|v| v.try_into().unwrap())
    }

    /// Iterate over journal entries, skipping `skip_count` of them
    ///
    /// Corresponds to `sd_journal_next_skip()`
    pub fn next_skip(&mut self, skip_count: u64) -> Result<u64> {
        crate::ffi_result(unsafe { ffi::sd_journal_next_skip(self.as_ptr(), skip_count) })
            .map(|v| v.try_into().unwrap())
    }

    /// Iterate in reverse over journal entries
    ///
    /// Corresponds to `sd_journal_previous()`
    pub fn previous(&mut self) -> Result<u64> {
        crate::ffi_result(unsafe { ffi::sd_journal_previous(self.as_ptr()) })
            .map(|v| v.try_into().unwrap())
    }

    /// Iterate in reverse over journal entries, skipping `skip_count` of them.
    ///
    /// Corresponds to `sd_journal_previous_skip()`
    pub fn previous_skip(&mut self, skip_count: u64) -> Result<usize> {
        crate::ffi_result(unsafe { ffi::sd_journal_previous_skip(self.as_ptr(), skip_count) })
            .map(|v| v.try_into().unwrap())
    }

    /// Read the next entry from the journal. Returns `Ok(None)` if there
    /// are no more entries to read.
    pub fn next_entry(&mut self) -> Result<Option<JournalRecord>> {
        if self.next()? == 0 {
            return Ok(None);
        }

        self.collect_entry().map(Some)
    }

    /// Read the previous entry from the journal. Returns `Ok(None)` if there
    /// are no more entries to read.
    pub fn previous_entry(&mut self) -> Result<Option<JournalRecord>> {
        if self.previous()? == 0 {
            return Ok(None);
        }

        self.collect_entry().map(Some)
    }

    /// Wait for next entry to arrive.
    /// Using a `wait_time` of `None` will wait for an unlimited period for new entries.
    ///
    /// Corresponds to `sd_journal_wait()`.
    pub fn wait(&mut self, wait_time: Option<time::Duration>) -> Result<JournalWaitResult> {
        let time = wait_time.map(usec_from_duration).unwrap_or(u64::MAX);

        match sd_try!(ffi::sd_journal_wait(self.as_ptr(), time)) {
            ffi::SD_JOURNAL_NOP => Ok(JournalWaitResult::Nop),
            ffi::SD_JOURNAL_APPEND => Ok(JournalWaitResult::Append),
            ffi::SD_JOURNAL_INVALIDATE => Ok(JournalWaitResult::Invalidate),
            _ => Err(io::Error::new(InvalidData, "Failed to wait for changes")),
        }
    }

    /// Wait for the next entry to appear. Returns `Ok(None)` if there were no
    /// new entries in the given wait time.
    /// Pass wait_time `None` to wait for an unlimited period for new entries.
    pub fn await_next_entry(
        &mut self,
        wait_time: Option<time::Duration>,
    ) -> Result<Option<JournalRecord>> {
        match self.wait(wait_time)? {
            JournalWaitResult::Nop => Ok(None),
            JournalWaitResult::Append => self.next_entry(),

            // This is possibly wrong, but I can't generate a scenario with
            // ..::Invalidate and neither systemd's journalctl,
            // systemd-journal-upload, and other utilities handle that case.
            JournalWaitResult::Invalidate => self.next_entry(),
        }
    }

    /// Iterate through all elements from the current cursor, then await the
    /// next entry(s) and wait again.
    pub fn watch_all_elements<F>(&mut self, mut f: F) -> Result<()>
    where
        F: FnMut(JournalRecord) -> Result<()>,
    {
        loop {
            let candidate = self.next_entry()?;
            let rec = match candidate {
                Some(rec) => rec,
                None => loop {
                    if let Some(r) = self.await_next_entry(None)? {
                        break r;
                    }
                },
            };
            f(rec)?
        }
    }

    /// Corresponds to `sd_journal_seek_head()`
    pub fn seek_head(&mut self) -> Result<()> {
        crate::ffi_result(unsafe { ffi::sd_journal_seek_head(self.as_ptr()) })?;

        Ok(())
    }

    /// Corresponds to `sd_journal_seek_tail()`
    pub fn seek_tail(&mut self) -> Result<()> {
        crate::ffi_result(unsafe { ffi::sd_journal_seek_tail(self.as_ptr()) })?;

        Ok(())
    }

    /// Corresponds to `sd_journal_seek_monotonic_usec()`
    pub fn seek_monotonic_usec(&mut self, boot_id: Id128, usec: u64) -> Result<()> {
        crate::ffi_result(unsafe {
            ffi::sd_journal_seek_monotonic_usec(self.as_ptr(), *boot_id.as_raw(), usec)
        })?;

        Ok(())
    }

    /// Corresponds to `sd_journal_seek_realtime_usec()`
    pub fn seek_realtime_usec(&mut self, usec: u64) -> Result<()> {
        crate::ffi_result(unsafe { ffi::sd_journal_seek_realtime_usec(self.as_ptr(), usec) })?;

        Ok(())
    }

    /// Corresponds to `sd_journal_seek_cursor()`
    pub fn seek_cursor<A: CStrArgument>(&mut self, cursor: A) -> Result<()> {
        let c = cursor.into_cstr();
        crate::ffi_result(unsafe {
            ffi::sd_journal_seek_cursor(self.as_ptr(), c.as_ref().as_ptr())
        })?;

        Ok(())
    }

    /// Seek to a specific position in journal using a general `JournalSeek`
    ///
    /// Note: after seeking, this [`Journal`] does not refer to any entry (and consequently can not
    /// obtain information about an entry, like [`cursor()`], etc). Use the iteration functions
    /// ([`next()`], [`previous()`], [`next_skip()`], and [`previous_skip()`]) to move onto an
    /// entry.
    pub fn seek(&mut self, seek: JournalSeek) -> Result<()> {
        match seek {
            JournalSeek::Head => self.seek_head()?,
            JournalSeek::Tail => {
                self.seek_tail()?;
            }
            JournalSeek::ClockMonotonic { boot_id, usec } => {
                self.seek_monotonic_usec(boot_id, usec)?;
            }
            JournalSeek::ClockRealtime { usec } => {
                self.seek_realtime_usec(usec)?;
            }
            JournalSeek::Cursor { cursor } => {
                self.seek_cursor(cursor)?;
            }
        };

        Ok(())
    }

    /// Returns the cursor of current journal entry.
    pub fn cursor(&self) -> Result<String> {
        let mut c_cursor: *const c_char = ptr::null_mut();

        sd_try!(ffi::sd_journal_get_cursor(self.as_ptr(), &mut c_cursor));
        let cursor = unsafe { free_cstring(c_cursor as *mut _).unwrap() };
        Ok(cursor)
    }

    /// Test if a given cursor matches the current postition in the journal
    ///
    /// Corresponds to `sd_journal_test_cursor()`.
    pub fn test_cursor<A: CStrArgument>(&self, cursor: A) -> Result<bool> {
        let c = cursor.into_cstr();
        crate::ffi_result(unsafe {
            ffi::sd_journal_test_cursor(self.as_ptr(), c.as_ref().as_ptr())
        })
        .map(|v| v != 0)
    }

    /// Returns timestamp at which current journal entry was recorded.
    pub fn timestamp(&self) -> Result<time::SystemTime> {
        let mut timestamp_us: u64 = 0;
        sd_try!(ffi::sd_journal_get_realtime_usec(
            self.as_ptr(),
            &mut timestamp_us
        ));
        Ok(system_time_from_realtime_usec(timestamp_us))
    }

    /// Returns monotonic timestamp and boot ID at which current journal entry was recorded.
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

    /// Returns monotonic timestamp at which current journal entry was recorded. Returns an error if
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
    pub fn match_add<T: Into<Vec<u8>>>(&mut self, key: &str, val: T) -> Result<&mut JournalRef> {
        let mut filter = Vec::<u8>::from(key);
        filter.push(b'=');
        filter.extend(val.into());
        let data = filter.as_ptr() as *const c_void;
        let datalen = filter.len() as size_t;
        sd_try!(ffi::sd_journal_add_match(self.as_ptr(), data, datalen));
        Ok(self)
    }

    /// Inserts a disjunction (i.e. logical OR) in the match list.
    pub fn match_or(&mut self) -> Result<&mut JournalRef> {
        sd_try!(ffi::sd_journal_add_disjunction(self.as_ptr()));
        Ok(self)
    }

    /// Inserts a conjunction (i.e. logical AND) in the match list.
    pub fn match_and(&mut self) -> Result<&mut JournalRef> {
        sd_try!(ffi::sd_journal_add_conjunction(self.as_ptr()));
        Ok(self)
    }

    /// Flushes all matches, disjunction and conjunction terms.
    /// After this call all filtering is removed and all entries in
    /// the journal will be iterated again.
    pub fn match_flush(&mut self) -> Result<&mut JournalRef> {
        unsafe { ffi::sd_journal_flush_matches(self.as_ptr()) };
        Ok(self)
    }
}

impl AsRawFd for JournalRef {
    #[inline]
    fn as_raw_fd(&self) -> c_int {
        self.fd().unwrap()
    }
}
