rust-systemd
============

In `Cargo.toml`:
```toml
[dependencies.systemd]
git = "https://github.com/jmesmon/rust-systemd"
```
Or
```toml
[dependencies]
systemd = "*"
```

journal
-------
Journal sending is supported, and systemd::journal::Journal is a (low
functionality) wrapper around the read API.

An example of the journal writing api:

```rust
#[macro_use] extern crate log;
#[macro_use] extern crate systemd;
use systemd::journal;

fn main() {
   use systemd::journal;
   journal::print(1, &format!("Rust can talk to the journal: {:?}",
                             4));
   journal::send(["CODE_FILE=HI", "CODE_LINE=1213", "CODE_FUNCTION=LIES"]);
   journal::JournalLogger::init().unwrap();
   warn!("HI");
   sd_journal_log!(4, "HI {:?}", 2);
}
```

daemon
------
The daemon API mostly offers tools for working with raw filehandles passed to
the process by systemd on socket activation. Since raw filehandles are not well
supported in Rust, it's likely these functions will mostly be helpful in
managing program flow; actual socket code will have to use the libc crate.

TODO
----

 - [ ] rustdoc
 - [ ] other systemd apis
 - [ ] pass travis automated tests
