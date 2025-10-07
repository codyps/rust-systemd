#![warn(rust_2018_idioms)]

#[cfg(feature = "journal")]
mod x {
    //! Demonstrate the new Result-based journal API with proper error handling.

    use systemd::journal;

    pub fn main() {
        println!("Demonstrating new Result-based journal API...");

        // Using try_send for detailed control
        match journal::try_send(&[
            "MESSAGE=Hello from Rust with Result API!",
            "PRIORITY=6",
            "CODE_FILE=journal-send-result.rs",
            "CODE_LINE=15",
            "CUSTOM_FIELD=test_value",
        ]) {
            Ok(()) => println!("Detailed message sent successfully"),
            Err(e) => println!("Failed to send detailed message: {}", e),
        }

        // Using try_print for simple messages
        if let Err(e) = journal::try_print(6, "Simple message with Result API") {
            println!("Failed to send simple message: {}", e);
        } else {
            println!("Simple message sent successfully");
        }

        // Using try_log for structured logging
        match journal::try_log(
            4, // Warning level
            file!(),
            line!(),
            module_path!(),
            &format_args!("Structured log entry with count: {}", 42),
        ) {
            Ok(()) => println!("Structured log sent successfully"),
            Err(e) => println!("Failed to send structured log: {}", e),
        }

        // Demonstrating error handling with invalid data
        // This should still succeed as systemd is quite permissive
        match journal::try_send(&["INVALID_KEY_WITHOUT_VALUE"]) {
            Ok(()) => println!("Even invalid-looking data was accepted"),
            Err(e) => println!("Invalid data was rejected: {}", e),
        }

        // Chaining operations with proper error handling
        let operations = [
            || journal::try_print(3, "Error level message"),
            || journal::try_print(4, "Warning level message"),
            || journal::try_print(6, "Info level message"),
            || journal::try_print(7, "Debug level message"),
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
        println!("Example completed. Check your journal with: journalctl -t journal-send-result");
    }
}

#[cfg(not(feature = "journal"))]
mod x {
    pub fn main() {
        println!("This example requires the 'journal' feature.");
        println!("Run with: cargo run --example journal-send-result --features journal");
    }
}

fn main() {
    x::main()
}
