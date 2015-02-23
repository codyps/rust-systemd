#[macro_use] extern crate systemd;
#[macro_use] extern crate log;

#[test]
fn test() {
    use systemd::journal;
    journal::send(&["CODE_FILE=HI", "CODE_LINE=1213", "CODE_FUNCTION=LIES"]);
    journal::print(1, format!("Rust can talk to the journal: {}",
                              4).as_slice());

    journal::JournalLog::init().ok().unwrap();
    log!(log::LogLevel::Info, "HI");
    sd_journal_log!(4, "HI {:?}", 2);
}

