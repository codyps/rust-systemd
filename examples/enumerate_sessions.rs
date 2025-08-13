// Copyright (C) 2025, Snap Inc.
//
// Example: Enumerate all systemd sessions and display their information
//
// This example demonstrates how to use the new session enumeration functions
// to list all active sessions and their properties.

extern crate systemd;

use std::time::{Duration, UNIX_EPOCH};
use systemd::login;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Enumerating systemd sessions...\n");

    // Get all active sessions
    let sessions = login::get_sessions()?;

    if sessions.is_empty() {
        println!("No active sessions found.");
        return Ok(());
    }

    println!("Found {} session(s):\n", sessions.len());

    for session_id in sessions {
        println!("Session: {}", session_id);

        // Get session UID
        if let Ok(uid) = login::get_session_uid(&session_id) {
            println!("  UID: {}", uid);
        }

        // Get session start time
        if let Ok(start_time_usec) = login::get_session_start_time(&session_id) {
            let start_time = UNIX_EPOCH + Duration::from_micros(start_time_usec);
            println!("  Start time: {:?}", start_time);
        }

        // Get session TTY (if available)
        if let Ok(Some(tty)) = login::get_session_tty(&session_id) {
            println!("  TTY: {}", tty);
        }

        // Get session display (if available)
        if let Ok(Some(display)) = login::get_session_display(&session_id) {
            println!("  Display: {}", display);
        }

        // Get session type
        if let Ok(Some(session_type)) = login::get_session_type(&session_id) {
            println!("  Type: {}", session_type);
        }

        // Get remote host (if available)
        if let Ok(Some(remote_host)) = login::get_session_remote_host(&session_id) {
            println!("  Remote host: {}", remote_host);
        }

        println!();
    }

    Ok(())
}
