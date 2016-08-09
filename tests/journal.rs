#[macro_use]
extern crate systemd;
#[macro_use]
extern crate log;

use std::path::Path;
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
    log!(log::LogLevel::Info, "HI");
    sd_journal_log!(4, "HI {:?}", 2);
}

#[test]
fn cursor() {
    if ! have_journal() {
        return;
    }

    let j = journal::Journal::open(journal::JournalFiles::All, false, false).unwrap();
    log!(log::LogLevel::Info, "rust-systemd test_seek entry");
    assert!(j.seek(journal::JournalSeek::Head).is_ok());
    let _s = j.cursor().unwrap();
}

#[test]
fn ts() {
    if ! have_journal() {
        return;
    }

    let j = journal::Journal::open(journal::JournalFiles::All, false, false).unwrap();
    log!(log::LogLevel::Info, "rust-systemd test_seek entry");
    assert!(j.seek(journal::JournalSeek::Head).is_ok());
    let _s = j.get_realtime_us().unwrap();
}


#[test]
fn test_seek() {
    let j = journal::Journal::open(journal::JournalFiles::All, false, false).unwrap();
    if ! have_journal() {
        return;
    }
    log!(log::LogLevel::Info, "rust-systemd test_seek entry");
    assert!(j.seek(journal::JournalSeek::Head).is_ok());
    assert!(j.next_record().is_ok());
    let c1 = j.seek(journal::JournalSeek::Current);
    assert!(c1.is_ok());
    let c2 = j.seek(journal::JournalSeek::Current);
    assert!(c2.is_ok());
    assert_eq!(c1.unwrap(), c2.unwrap());
    assert!(j.seek(journal::JournalSeek::Tail).is_ok());
    assert!(j.next_record().is_ok());
}
