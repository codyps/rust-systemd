//! Follow future journal log messages and print up to 100 of them.

extern crate systemd;

use std::io::ErrorKind;

use systemd::Error;
use systemd::journal::{Journal, JournalFiles, JournalRecord, JournalSeek};

const KEY_UNIT: &str = "_SYSTEMD_UNIT";
const KEY_MESSAGE: &str = "MESSAGE";

const MAX_MESSAGES: usize = 100;

fn main() {
    println!("Starting journal-logger");

    // Open the journal
    let runtime_only = false;
    let local_only = false;
    let mut reader = Journal::open(&JournalFiles::All, runtime_only, local_only)
        .expect("Could not open journal");

    // Seek to end of current log to prevent old messages from being printed
    reader.seek(JournalSeek::Tail)
        .expect("Could not seek to end of journal");

    // Print up to MAX_MESSAGES incoming messages
    let mut i = 0;
    reader.watch_all_elements(|record: JournalRecord| {
        let unit = record.get(KEY_UNIT)
            .ok_or_else(|| Error::new(ErrorKind::Other, "Could not get unit from record"))?;
        let message = record.get(KEY_MESSAGE)
            .ok_or_else(|| Error::new(ErrorKind::Other, "Could not get message from record"))?;
        println!("[{}] {}", unit, message);

        i += 1;
        if i < MAX_MESSAGES {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::Other, "Done watching"))
        }
    }).unwrap_or_else(|e| {
        println!("Stop watching log. Reason: {}", e);
    });

    println!("End of example.");
}
