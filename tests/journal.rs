#[macro_use]
extern crate systemd;
#[macro_use]
extern crate log;
use systemd::journal::{self, Journal, JournalFiles};

#[test]
fn test() {
    journal::send(&["CODE_FILE=HI", "CODE_LINE=1213", "CODE_FUNCTION=LIES"]);
    journal::print(1, &format!("Rust can talk to the journal: {}", 4));

    journal::JournalLog::init().ok().unwrap();
    log!(log::LogLevel::Info, "HI");
    sd_journal_log!(4, "HI {:?}", 2);
}

// #[test]
fn iterator_test() {
    let mut client = match Journal::open(JournalFiles::All, true, true) {
        Ok(c) => c,
        Err(e) => {
            println!("Error opening");
            panic!("Couldn't create client. Error = {:?}", e);
        }
    };
    let mut count = 0;
    for (journal, cursor) in client {
        count += 1;
        println!("{:?}", count);
    }
}
