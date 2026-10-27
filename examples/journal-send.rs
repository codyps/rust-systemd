#![warn(rust_2018_idioms)]

#[cfg(feature = "journal")]
mod x {
    //! Demonstrate the new Result-based journal API with proper error handling.

    use systemd::journal;

    pub fn main() {
        println!("Demonstrating new Result-based journal API...");
        println!("Success means libsystemd reported no error; it does not acknowledge delivery.");

        // Using send for detailed control
        match journal::send(&[
            "MESSAGE=Hello from Rust with Result API!",
            "PRIORITY=6",
            "CODE_FILE=journal-send.rs",
            "CODE_LINE=15",
            "CUSTOM_FIELD=test_value",
        ]) {
            Ok(()) => println!("libsystemd accepted the detailed message"),
            Err(e) => println!("Failed to send detailed message: {}", e),
        }

        // Using print for simple messages
        if let Err(e) = journal::print(6, "Simple message with Result API") {
            println!("Failed to send simple message: {}", e);
        } else {
            println!("libsystemd accepted the simple message");
        }

        // Using log for structured logging
        match journal::log(
            4, // Warning level
            file!(),
            line!(),
            module_path!(),
            &format_args!("Structured log entry with count: {}", 42),
        ) {
            Ok(()) => println!("libsystemd accepted the structured log"),
            Err(e) => println!("Failed to send structured log: {}", e),
        }

        // An empty field list is rejected with EINVAL.
        match journal::send(&[]) {
            Ok(()) => println!("Unexpectedly accepted an empty field list"),
            Err(e) => println!("Empty field list was rejected: {}", e),
        }

        // Chaining operations with proper error handling
        let operations = [
            || journal::print(3, "Error level message"),
            || journal::print(4, "Warning level message"),
            || journal::print(6, "Info level message"),
            || journal::print(7, "Debug level message"),
        ];

        let mut success_count = 0;
        for (i, op) in operations.iter().enumerate() {
            match op() {
                Ok(()) => {
                    success_count += 1;
                    println!("Operation {} succeeded", i + 1);
                }
                Err(e) => println!("Operation {} failed: {}", i + 1, e),
            }
        }

        println!(
            "Summary: {}/{} operations succeeded",
            success_count,
            operations.len()
        );
        println!(
            "Example completed. Check your journal with: journalctl _PID={}",
            std::process::id()
        );
    }
}

#[cfg(not(feature = "journal"))]
mod x {
    pub fn main() {
        println!("This example requires the 'journal' feature.");
        println!("Run with: cargo run --example journal-send --features journal");
    }
}

fn main() {
    x::main()
}
