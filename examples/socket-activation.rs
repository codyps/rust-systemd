//! # Using systemd:
//!
//! `example.socket`:
//! ```
//! [Socket]
//! ListenStream=5000 # pick your port
//!
//! [Install]
//! WantedBy=sockets.target
//! ```
//!
//! `example.service`:
//! ```
//! [Unit]
//! Requires=example.socket
//!
//! [Service]
//! ExecStart=/opt/rust-systemd/bin/example-socket-activation
//!
//! [Install]
//! WantedBy=multi-user.target
//! ```
//!
//!
//! # Alternately, using `systemfd`
//!
//! ```
//! systemfd -s 5000 -- cargo run --example socket-activation
//! ```

#![warn(rust_2018_idioms)]

use std::io::Write;
use std::net::TcpStream;
use systemd::daemon;

fn handle_client(mut stream: TcpStream) {
    stream.write_all(b"HI\n").unwrap();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lfds = daemon::listen_fds(false).unwrap_or(0);
    if lfds != 1 {
        panic!("Must have exactly 1 fd to listen on, got {}", lfds);
    }

    let listener = daemon::tcp_listener(daemon::LISTEN_FDS_START).unwrap();

    // accept connections and process them serially
    for stream in listener.incoming() {
        handle_client(stream?);
    }
    Ok(())
}
