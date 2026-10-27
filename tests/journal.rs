#![cfg(feature = "journal")]
#![warn(rust_2018_idioms)]

use log::log;
use systemd::sd_journal_log;

use std::path::Path;
use systemd::id128;
use systemd::journal;

// Some systems don't have a running journal, which causes our tests to fail currently
//
// TODO: adjust tests that use this to generate a fixed journal if possible, or ship some test
// data.
fn have_journal() -> bool {
    if !Path::new("/run/systemd/journal/").exists() {
        println!("missing journal files");
        false
    } else {
        true
    }
}

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
    if !have_journal() {
        return;
    }

    let mut j = journal::OpenOptions::default().open().unwrap();
    log!(log::Level::Info, "rust-systemd test_seek entry");
    j.seek(journal::JournalSeek::Head).unwrap();
    j.next().unwrap();
    let _s = j.cursor().unwrap();
}

#[test]
fn ts() {
    if !have_journal() {
        return;
    }

    let mut j = journal::OpenOptions::default().open().unwrap();
    log!(log::Level::Info, "rust-systemd ts entry");
    j.seek(journal::JournalSeek::Head).unwrap();
    j.next().unwrap();
    let _s = j.timestamp().unwrap();
    j.seek(journal::JournalSeek::Tail).unwrap();
    j.previous().unwrap();
    let (u1, entry_boot_id) = j.monotonic_timestamp().unwrap();
    assert!(u1 > 0);
    let boot_id = id128::Id128::from_boot().unwrap();
    assert!(boot_id == entry_boot_id);
    let u2 = j.monotonic_timestamp_current_boot().unwrap();
    assert_eq!(u1, u2);
}

#[test]
fn test_timestamp() {
    if !have_journal() {
        return;
    }

    let mut j = journal::OpenOptions::default().open().unwrap();
    log!(log::Level::Info, "rust-systemd ts entry");
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
    let mut j = journal::OpenOptions::default().open().unwrap();
    if !have_journal() {
        return;
    }
    log!(log::Level::Info, "rust-systemd test_seek entry");
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
    if !have_journal() {
        return;
    }
    let key = "RUST_TEST_MARKER";
    let value = "RUST_SYSTEMD_SIMPLE_MATCH";
    let msg = "MESSAGE=rust-systemd test_match";
    let filter = format!("{key}={value}");
    let mut j = journal::OpenOptions::default().open().unwrap();

    // check for positive matches

    // seek tail
    j.seek(journal::JournalSeek::Tail).unwrap();
    j.previous().unwrap();
    j.match_add(key, value).unwrap();

    journal::send(&[&filter, msg]).unwrap();
    let mut waits = 0;
    loop {
        if j.next().unwrap() == 0 {
            if waits > 5 {
                panic!("got to end of journal without finding our entry");
            }

            waits += 1;
            j.wait(Some(std::time::Duration::from_secs(1))).unwrap();
            continue;
        }

        let entryval = j.get_data(key).unwrap();
        println!("k,v: {key:?}, {entryval:?}");
        if entryval.is_none() {
            println!("E: {}", j.display_entry_data());
            continue;
        }

        assert_eq!(entryval.unwrap().value().unwrap(), value.as_bytes());
        break;
    }

    // check for negative matches
    j.seek(journal::JournalSeek::Tail).unwrap();
    j.previous().unwrap();
    j.match_flush()
        .unwrap()
        .match_add("NOKEY", "NOVALUE")
        .unwrap();
    journal::send(&[msg]).unwrap();
    while j.next().unwrap() != 0 {
        assert!(j.get_data("NO_KEY").unwrap().is_none())
    }
}

#[test]
fn get_data() {
    if !have_journal() {
        return;
    }

    let mut j = journal::OpenOptions::default().open().unwrap();
    j.seek_tail().unwrap();
    journal::send(&["RUST_TEST_MARKER=1"]).unwrap();
    j.match_add("RUST_TEST_MARKER", "1").unwrap();

    loop {
        if j.next().unwrap() == 0 {
            break;
        }

        assert_eq!(j.get_data("THIS_DOES_NOT_EXIST").unwrap(), None);
    }
}

#[test]
fn journal_entry_data_1() {
    let jrd: journal::JournalEntryField<'_> = b"HI=foo"[..].into();

    assert_eq!(jrd.data(), &b"HI=foo"[..]);
    assert_eq!(jrd.name(), &b"HI"[..]);
    assert_eq!(jrd.value(), Some(&b"foo"[..]));
}
