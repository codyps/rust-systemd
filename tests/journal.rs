#![feature(phase)]

#[phase(plugin,link)] extern crate systemd;
#[phase(plugin,link)] extern crate log;

#[test]
fn test() {
    use log::{set_logger};
    use systemd::journal;
    journal::send(&["CODE_FILE=HI", "CODE_LINE=1213", "CODE_FUNCTION=LIES"]);
    journal::print(1, format!("Rust can talk to the journal: {}",
                              4i).as_slice());
    set_logger(box journal::JournalLogger);
    log!(0, "HI");
    sd_journal_log!(4, "HI {}", 2i);
}

