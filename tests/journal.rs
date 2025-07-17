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
