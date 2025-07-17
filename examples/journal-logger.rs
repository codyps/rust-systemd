#![warn(rust_2018_idioms)]

#[cfg(feature = "journal")]
mod x {
    //! Follow future journal log messages and print up to 100 of them.
    

    use systemd::journal::{self, JournalRecord, JournalSeek};
    use systemd::Error;

    const KEY_UNIT: &str = "_SYSTEMD_UNIT";
    const KEY_MESSAGE: &str = "MESSAGE";

    const MAX_MESSAGES: usize = 100;

    pub fn main() {
        println!("Starting journal-logger");

        // Open the journal
        let mut reader = journal::OpenOptions::default()
            .open()
            .expect("Could not open journal");

        // Seek to end of current log to prevent old messages from being printed
        reader
            .seek(JournalSeek::Tail)
            .expect("Could not seek to end of journal");

        // Print up to MAX_MESSAGES incoming messages
        let mut i = 0;
        reader
            .watch_all_elements(|record: JournalRecord| {
                let unit = record.get(KEY_UNIT).ok_or_else(|| {
                    Error::other("Could not get unit from record")
                })?;
                let message = record.get(KEY_MESSAGE).ok_or_else(|| {
                    Error::other("Could not get message from record")
                })?;
                println!("[{unit}] {message}");

                i += 1;
                if i < MAX_MESSAGES {
                    Ok(())
                } else {
                    Err(Error::other("Done watching"))
                }
            })
            .unwrap_or_else(|e| {
                println!("Stop watching log. Reason: {e}");
            });

        println!("End of example.");
    }
}

#[cfg(not(feature = "journal"))]
mod x {
    pub fn main() {
        println!("pass `--features journal`");
    }
}

fn main() {
    x::main()
}
