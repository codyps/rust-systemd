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
fn test() {
    journal::send(&["CODE_FILE=HI", "CODE_LINE=1213", "CODE_FUNCTION=LIES"]);
    journal::print(1, &format!("Rust can talk to the journal: {}", 4));

    journal::JournalLog::init().ok().unwrap();
    log::set_max_level(log::LevelFilter::Warn);
    log!(log::Level::Info, "HI info");
    log!(target: "systemd-tests", log::Level::Info, "HI info with target");
    log!(log::Level::Warn, "HI warn");
    log!(target: "systemd-tests", log::Level::Warn, "HI warn with target");
    sd_journal_log!(4, "HI {:?}", 2);
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

    journal::send(&[&filter, msg]);
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
    journal::send(&[msg]);
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
    journal::send(&["RUST_TEST_MARKER=1"]);
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
