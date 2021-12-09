#![warn(rust_2018_idioms)]

#[cfg(feature = "journal")]
mod x {
    //! Follow future journal log messages and print up to 100 of them.
    use systemd::journal::{self, JournalSeek};

    const KEY_UNIT: &str = "_SYSTEMD_UNIT";
    const KEY_MESSAGE: &str = "MESSAGE";

    const MAX_MESSAGES: usize = 100;

    pub fn main() -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting journal-logger");

        // Open the journal
        let mut reader = journal::OpenOptions::default()
            .open()
            .expect("Could not open journal");

        // Seek to end of current log to prevent old messages from being printed
        reader
            .seek(JournalSeek::Tail)
            .expect("Could not seek to end of journal");

        // JournalSeek::Tail goes to the position after the most recent entry so step back to
        // point to the most recent entry.
        reader.previous()?;

        // Print up to MAX_MESSAGES incoming messages
        let mut i = 0;
        loop {
            loop {
                if reader.next()? == 0 {
                    break;
                }

                let unit: Option<_> = reader.get_data(KEY_UNIT)?.and_then(|v| {
                    v.value()
                        .map(String::from_utf8_lossy)
                        .map(|v| v.into_owned())
                });
                let message = reader.get_data(KEY_MESSAGE)?.and_then(|v| {
                    v.value()
                        .map(String::from_utf8_lossy)
                        .map(|v| v.into_owned())
                });

                println!("[{:?}] {:?}", unit, message);

                i += 1;
                if i >= MAX_MESSAGES {
                    eprintln!("done.");
                    return Ok(());
                }
            }

            reader.wait(None)?;
        }
    }
}

#[cfg(not(feature = "journal"))]
mod x {
    pub fn main() -> Result<(), Box<dyn std::error::Error>> {
        println!("pass `--features journal`");
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    x::main()
}
