#[macro_use]
extern crate systemd;
#[macro_use]
extern crate log;

use std::path::Path;
use systemd::journal;

#[test]
fn test() {
    journal::send(&["CODE_FILE=HI", "CODE_LINE=1213", "CODE_FUNCTION=LIES"]);
    journal::print(1, &format!("Rust can talk to the journal: {}", 4));

    journal::JournalLog::init().ok().unwrap();
    log!(log::LogLevel::Info, "HI");
    sd_journal_log!(4, "HI {:?}", 2);
}

#[test]
fn test_seek() {
    let j = journal::Journal::open(journal::JournalFiles::All, false, false).unwrap();
    if !Path::new("/run/systemd/journal/").exists() {
        println!("missing journal files");
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
