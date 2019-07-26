#[macro_use]
extern crate systemd;
#[macro_use]
extern crate log;

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

    let mut j = journal::Journal::open(journal::JournalFiles::All, false, false).unwrap();
    log!(log::Level::Info, "rust-systemd test_seek entry");
    assert!(j.seek(journal::JournalSeek::Head).is_ok());
    let _s = j.cursor().unwrap();
}

#[test]
fn ts() {
    if !have_journal() {
        return;
    }

    let mut j = journal::Journal::open(journal::JournalFiles::All, false, false).unwrap();
    log!(log::Level::Info, "rust-systemd test_seek entry");
    assert!(j.seek(journal::JournalSeek::Head).is_ok());
    let _s = j.timestamp().unwrap();
    assert!(j.seek(journal::JournalSeek::Tail).is_ok());
    let (u1, entry_boot_id) = j.monotonic_timestamp().unwrap();
    assert!(u1 > 0);
    let boot_id = id128::Id128::from_boot().unwrap();
    assert!(boot_id == entry_boot_id);
    let u2 = j.monotonic_timestamp_current_boot().unwrap();
    assert_eq!(u1, u2);
}

#[test]
fn test_seek() {
    let mut j = journal::Journal::open(journal::JournalFiles::All, false, false).unwrap();
    if !have_journal() {
        return;
    }
    log!(log::Level::Info, "rust-systemd test_seek entry");
    assert!(j.seek(journal::JournalSeek::Head).is_ok());
    assert!(j.next_record().is_ok());
    let c1 = j.seek(journal::JournalSeek::Current);
    assert!(c1.is_ok());
    let c2 = j.seek(journal::JournalSeek::Current);
    assert!(c2.is_ok());
    assert_eq!(c1.unwrap(), c2.unwrap());
    assert!(j.seek(journal::JournalSeek::Tail).is_ok());
    assert!(j.next_record().is_ok());
    let c3 = j.cursor().unwrap();
    let valid_cursor = journal::JournalSeek::Cursor { cursor: c3 };
    assert!(j.seek(valid_cursor).is_ok());
    let invalid_cursor = journal::JournalSeek::Cursor {
        cursor: "".to_string(),
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
    let filter = format!("{}={}", key, value);
    let mut j = journal::Journal::open(journal::JournalFiles::All, false, false).unwrap();

    // check for positive matches
    assert!(j.seek(journal::JournalSeek::Tail).is_ok());
    journal::send(&[&filter, &msg]);
    assert!(j.match_flush().unwrap().match_add(key, value).is_ok());
    let r = j.next_record().unwrap();
    assert!(r.is_some());
    let entry = r.unwrap();
    let entryval = entry.get(key);
    assert!(entryval.is_some());
    assert_eq!(entryval.unwrap(), value);

    // check for negative matches
    assert!(j.seek(journal::JournalSeek::Tail).is_ok());
    assert!(j
        .match_flush()
        .unwrap()
        .match_add("NOKEY", "NOVALUE")
        .is_ok());
    journal::send(&[&msg]);
    assert!(j.next_record().unwrap().is_none());
}
