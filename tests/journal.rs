#![cfg(feature = "journal")]
#![warn(rust_2018_idioms)]

use log::log;
use systemd::sd_journal_log;

use systemd::id128;
use systemd::journal;

#[test]
#[deny(deprecated)]
fn test() -> systemd::Result<()> {
    journal::send(&["CODE_FILE=HI", "CODE_LINE=1213", "CODE_FUNCTION=LIES"])?;
    journal::print(1, &format!("Rust can talk to the journal: {}", 4))?;

    journal::JournalLog::init().ok().unwrap();
    log::set_max_level(log::LevelFilter::Warn);
    log!(log::Level::Info, "HI info");
    log!(target: "systemd-tests", log::Level::Info, "HI info with target");
    log!(log::Level::Warn, "HI warn");
    log!(target: "systemd-tests", log::Level::Warn, "HI warn with target");
    sd_journal_log!(4, "HI {:?}", 2)?;
    Ok(())
}

#[test]
fn test_result_api() {
    // Test the Result-returning API
    journal::send(&["CODE_FILE=HI", "CODE_LINE=1213", "CODE_FUNCTION=LIES"]).unwrap();
    journal::print(1, &format!("Rust can talk to the journal: {}", 4)).unwrap();

    // Test that the functions return Ok(()) on success
    let result1 = journal::send(&["MESSAGE=test message"]);
    assert!(result1.is_ok());

    let result2 = journal::print(6, "test print");
    assert!(result2.is_ok());

    // Test log function
    let result3 = journal::log(
        6,
        "test.rs",
        42,
        "test_module",
        &format_args!("test log entry"),
    );
    assert!(result3.is_ok());
}

#[test]
fn test_api_with_special_chars() {
    let result = journal::send(&[
        "MESSAGE=Test with special chars: éàü©",
        "CUSTOM=Line\nbreak",
    ]);
    assert!(result.is_ok());
}

#[test]
fn test_empty_args() {
    let error = journal::send(&[]).unwrap_err();
    assert_eq!(error.raw_os_error(), Some(libc::EINVAL));
}

#[test]
fn send_validation_matches_native() {
    let cases: &[(&[&str], libc::c_int)] = &[
        (&[], -libc::EINVAL),
        (&[""], -libc::EINVAL),
        (&["X"], -libc::EINVAL),
        (&["MISSING_EQUALS"], -libc::EINVAL),
        (&["=missing_name"], -libc::EINVAL),
        (&["BAD\nNAME=value"], -libc::EINVAL),
        (&["MESSAGE=valid", "INVALID"], -libc::EINVAL),
        (&["MESSAGE="], 0),
        (&["MESSAGE=Unicode: éàü©"], 0),
        (&["MESSAGE=line one\nline two\n"], 0),
        (&["MESSAGE=embedded\0NUL"], 0),
        (&["CUSTOM=one", "CUSTOM=two"], 0),
    ];

    for &(fields, expected) in cases {
        let iovecs: Vec<_> = fields
            .iter()
            // SAFETY: fields stay alive until all native calls in this iteration have returned.
            .map(|field| unsafe { libsystemd_sys::const_iovec::from_str(*field) })
            .collect();
        // SAFETY: iovecs references live fields, and every test case has a small field count.
        let native = unsafe {
            libsystemd_sys::journal::sd_journal_sendv(iovecs.as_ptr(), iovecs.len() as libc::c_int)
        };
        assert_eq!(native, expected, "native result for {fields:?}");
        let result = journal::send(fields);
        if native < 0 {
            assert_eq!(
                result.unwrap_err().raw_os_error(),
                Some(-native),
                "Result errno for {fields:?}"
            );
        } else {
            result.unwrap();
        }
    }
}

#[test]
fn test_log_record() {
    let record = log::Record::builder()
        .args(format_args!("Test structured record"))
        .level(log::Level::Warn)
        .target("systemd-tests")
        .file(Some(file!()))
        .line(Some(line!()))
        .module_path(Some(module_path!()))
        .build();
    journal::log_record(&record).unwrap();

    // Records from other logging implementations can omit source locations.
    let record = log::Record::builder()
        .args(format_args!("Test record without source location"))
        .build();
    journal::log_record(&record).unwrap();
}

#[test]
#[cfg(target_os = "linux")]
fn journal_errors_are_returned() {
    const CHILD_ENV: &str = "RUST_SYSTEMD_TEST_JOURNAL_ERRORS";
    if std::env::var_os(CHILD_ENV).is_none() {
        // A fresh process isolates both the descriptor limit and libsystemd's cached journal fd.
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "journal_errors_are_returned", "--nocapture"])
            .env(CHILD_ENV, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "child failed: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    let limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: this changes the descriptor limit only in this isolated child process.
    assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &limit) }, 0);

    let fields = &["MESSAGE=Test descriptor exhaustion"];
    let args = format_args!("Test descriptor exhaustion");
    let record = log::Record::builder().args(args).build();
    for result in [
        journal::send(fields),
        journal::print(6, "Test descriptor exhaustion"),
        journal::log(6, file!(), line!(), module_path!(), &args),
        journal::log_record(&record),
        sd_journal_log!(6, "Test descriptor exhaustion"),
    ] {
        assert_eq!(result.unwrap_err().raw_os_error(), Some(libc::EMFILE));
    }

    // The log facade has no Result return, so its adapter remains best-effort.
    log::Log::log(&journal::JournalLog, &record);
}

#[test]
#[deny(deprecated)]
fn log_macro_returns_result() {
    let result: systemd::Result<()> = sd_journal_log!(4, "Test macro with argument {}", 42);
    result.unwrap();
}

#[test]
fn cursor() {
    let mut j = fixture_journal();
    j.seek(journal::JournalSeek::Head).unwrap();
    j.next().unwrap();
    let _s = j.cursor().unwrap();
}

#[test]
fn ts() {
    let mut j = fixture_journal();
    j.seek(journal::JournalSeek::Head).unwrap();
    j.next().unwrap();
    let _s = j.timestamp().unwrap();
    j.seek(journal::JournalSeek::Tail).unwrap();
    j.previous().unwrap();
    let (u1, entry_boot_id) = j.monotonic_timestamp().unwrap();
    assert_eq!(u1, 3_000_000);
    assert_eq!(entry_boot_id, fixture_boot_id());
    if id128::Id128::from_boot().unwrap() == fixture_boot_id() {
        assert_eq!(j.monotonic_timestamp_current_boot().unwrap(), u1);
    } else {
        assert_eq!(
            j.monotonic_timestamp_current_boot()
                .unwrap_err()
                .raw_os_error(),
            Some(libc::ESTALE)
        );
    }
}

#[test]
fn empty_journal() {
    let mut j = journal::OpenFilesOptions::default()
        .open_files(std::iter::empty::<&str>())
        .unwrap();

    assert_eq!(j.cutoff_realtime_usec().unwrap(), None);
    assert_eq!(j.cutoff_realtime().unwrap(), None);
    assert_eq!(
        j.cutoff_monotonic_usec(id128::Id128::from_boot().unwrap())
            .unwrap(),
        None
    );
    assert_eq!(j.cutoff_monotonic_usec_current_boot().unwrap(), None);
    assert_eq!(j.usage().unwrap(), 0);
    assert!(!j.has_persistent_files().unwrap());
    assert!(!j.has_runtime_files().unwrap());

    j.query_unique("REVIEW_VALUE").unwrap();
    assert_eq!(j.enumerate_unique().unwrap(), None);
    j.restart_unique();
    assert_eq!(j.enumerate_unique().unwrap(), None);
}

#[test]
fn test_timestamp() {
    let mut j = fixture_journal();
    j.seek(journal::JournalSeek::Head).unwrap();
    j.next().unwrap();
    let timestamp_system_time = j.timestamp().unwrap();
    let timestamp_usec = j.timestamp_usec().unwrap();

    let since_the_epoch = timestamp_system_time
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards");
    let timestamp_system_time_usec = since_the_epoch.as_micros() as u64;

    assert_eq!(timestamp_usec, timestamp_system_time_usec);
}

#[test]
fn test_seek() {
    let mut j = fixture_journal();
    j.seek(journal::JournalSeek::Head).unwrap();
    j.next_entry().unwrap();
    let c1 = j.cursor().unwrap();
    let c2 = j.cursor().unwrap();
    assert_eq!(c1, c2);

    j.seek(journal::JournalSeek::Tail).unwrap();
    // NOTE: depending on the libsystemd version we may or may not be able to read an entry
    // following the "Tail", so ignore it.
    j.next_entry().unwrap();

    let valid_cursor = journal::JournalSeek::Cursor { cursor: c1 };
    j.seek(valid_cursor).unwrap();
    let invalid_cursor = journal::JournalSeek::Cursor {
        cursor: "invalid".to_string(),
    };
    assert!(j.seek(invalid_cursor).is_err());
}

#[test]
fn test_simple_match() {
    let key = "REVIEW_GROUP";
    let value = "one";
    let mut j = fixture_journal();
    j.match_add(key, value).unwrap();
    let mut matches = 0;
    while j.next().unwrap() != 0 {
        assert_eq!(
            j.get_data(key).unwrap().unwrap().value(),
            Some(value.as_bytes())
        );
        matches += 1;
    }
    assert_eq!(matches, 2);
    j.seek_head().unwrap();
    j.match_flush()
        .unwrap()
        .match_add("NOKEY", "NOVALUE")
        .unwrap();
    assert_eq!(j.next().unwrap(), 0);
}

#[test]
fn get_data() {
    let mut j = fixture_journal();
    let mut entries = 0;
    while j.next().unwrap() != 0 {
        assert_eq!(j.get_data("THIS_DOES_NOT_EXIST").unwrap(), None);
        entries += 1;
    }
    assert_eq!(entries, 3);
}

#[test]
fn journal_entry_data_1() {
    let jrd: journal::JournalEntryField<'_> = b"HI=foo"[..].into();

    assert_eq!(jrd.data(), &b"HI=foo"[..]);
    assert_eq!(jrd.name(), &b"HI"[..]);
    assert_eq!(jrd.value(), Some(&b"foo"[..]));
}

fn fixture_journal() -> journal::Journal {
    journal::OpenFilesOptions::default()
        .open_files([concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/journal.journal"
        )])
        .unwrap()
}

fn fixture_boot_id() -> id128::Id128 {
    id128::Id128::from_cstr(
        std::ffi::CStr::from_bytes_with_nul(b"11111111111111111111111111111111\0").unwrap(),
    )
    .unwrap()
}

#[test]
fn finite_wait_timeouts_return_overflow_errors() {
    let mut journal = fixture_journal();
    for timeout in [
        std::time::Duration::from_micros(u64::MAX),
        std::time::Duration::from_micros(u64::MAX) + std::time::Duration::from_micros(1),
        std::time::Duration::MAX,
    ] {
        assert_eq!(
            journal.wait(Some(timeout)).unwrap_err().raw_os_error(),
            Some(libc::EOVERFLOW)
        );
    }
    journal.wait(Some(std::time::Duration::ZERO)).unwrap();
}

#[test]
fn await_next_entry_drains_available_entries() {
    let mut j = fixture_journal();
    for message in [
        "first fixture entry",
        "second fixture entry",
        "third fixture entry",
    ] {
        let record = j.await_next_entry(Some(std::time::Duration::ZERO)).unwrap();
        assert_eq!(record.unwrap()["MESSAGE"], message);
    }
    assert!(j
        .await_next_entry(Some(std::time::Duration::ZERO))
        .unwrap()
        .is_none());
}

#[test]
fn await_next_entry_honors_timeout_after_file_notifications() {
    let mut j = fixture_journal();
    j.match_add("NO_MATCHING_FIELD", "value").unwrap();
    let timeout = std::time::Duration::from_millis(60);
    let started = std::time::Instant::now();
    assert!(j.await_next_entry(Some(timeout)).unwrap().is_none());
    assert!(started.elapsed() >= timeout);
}

#[test]
fn await_next_entry_without_timeout_waits_for_matching_data() {
    let directory =
        std::env::temp_dir().join(format!("rust-systemd-journal-wait-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let cleanup = Cleanup(directory);
    let mut j = journal::OpenDirectoryOptions::default()
        .open_directory(cleanup.0.to_str().unwrap())
        .unwrap();
    let destination = cleanup.0.join("system.journal");
    let writer = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(60));
        // Rename a complete journal into place so the reader observes a file
        // invalidation event followed by matching entries, without journald.
        let staging = destination.with_extension("staging");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/journal.journal"
            ),
            &staging,
        )
        .unwrap();
        std::fs::rename(staging, destination).unwrap();
    });
    let record = j.await_next_entry(None).unwrap();
    writer.join().unwrap();
    assert_eq!(record.unwrap()["MESSAGE"], "first fixture entry");
}

#[test]
fn cutoff_timestamps() {
    let j = fixture_journal();
    let (from, to) = (1_700_000_000_000_000, 1_700_000_002_000_000);
    assert_eq!(j.cutoff_realtime_usec().unwrap(), Some((from, to)));
    assert_eq!(
        j.cutoff_realtime().unwrap(),
        Some((
            std::time::UNIX_EPOCH + std::time::Duration::from_micros(from),
            std::time::UNIX_EPOCH + std::time::Duration::from_micros(to),
        ))
    );
    assert_eq!(
        j.cutoff_monotonic_usec(fixture_boot_id()).unwrap(),
        Some((1_000_000, 3_000_000))
    );
}

#[test]
fn cutoff_missing_boot() {
    let mut j = fixture_journal();
    // The journal is nonempty, but has no entries for the null boot ID.
    assert_eq!(j.next().unwrap(), 1);
    assert_eq!(
        j.cutoff_monotonic_usec(id128::Id128::default()).unwrap(),
        None
    );

    let expected = if id128::Id128::from_boot().unwrap() == fixture_boot_id() {
        Some((1_000_000, 3_000_000))
    } else {
        None
    };
    assert_eq!(j.cutoff_monotonic_usec_current_boot().unwrap(), expected);
}

#[test]
fn unique_values_ignore_matches() {
    fn collect_unique(j: &mut journal::JournalRef) -> std::collections::BTreeSet<Vec<u8>> {
        let mut values = std::collections::BTreeSet::new();
        while let Some(field) = j.enumerate_unique().unwrap() {
            assert_eq!(field.name(), b"REVIEW_VALUE");
            assert!(values.insert(field.value().unwrap().to_vec()));
        }
        values
    }

    let mut j = fixture_journal();
    assert_eq!(
        j.enumerate_unique().unwrap_err().raw_os_error(),
        Some(libc::EINVAL)
    );
    j.match_add("REVIEW_GROUP", "one").unwrap();
    j.query_unique("REVIEW_VALUE").unwrap();
    // Only alpha occurs in group one, but unique enumeration ignores the match.
    let expected = [b"alpha".to_vec(), b"beta".to_vec()]
        .iter()
        .cloned()
        .collect();
    assert_eq!(collect_unique(&mut j), expected);
    j.restart_unique();
    assert_eq!(collect_unique(&mut j), expected);

    j.query_unique("REVIEW_MISSING").unwrap();
    assert_eq!(j.enumerate_unique().unwrap(), None);
    assert_eq!(
        j.query_unique("INVALID=FIELD").unwrap_err().raw_os_error(),
        Some(libc::EINVAL)
    );
}
