rust-systemd
============

[crate docs (systemd)](http://docs.rs/crate/systemd)
[crate docs (libsystemd-sys)](http://docs.rs/crate/libsystemd-sys)
[![Crates.io](https://img.shields.io/crates/v/systemd.svg?maxAge=2592000)](https://crates.io/crates/systemd)
[![Build Status](https://travis-ci.org/jmesmon/rust-systemd.svg?branch=master)](https://travis-ci.org/jmesmon/rust-systemd)


In `Cargo.toml`:
```toml
[dependencies]
systemd = "0.5"
```

Build Environment variables
---------------------------

By default, `libsystemd-sys` will use `pkg-config` to find `libsystemd`. It
defaults to using the `systemd` package. To change the package looked up in
pkg-config, set the `SYSTEMD_PKG_NAME` environment variable.

If you want to override the source of the `libsystemd` directly, set the env
var `SYSTEMD_LIB_DIR` to a path which contains the `libsystemd` to link
against. Optionally, you may also set `SYSTEMD_LIBS` to indicate which
libraries to link against. Libraries in the variable `SYSTEMD_LIBS` are colon
(`:`) seperated and may include a `KIND`. For example:
`SYSTEMD_LIBS="static=foo:bar"`.


elogind support
---------------

Either set `SYSTEMD_PKG_NAME=elogind` or set both `SYSTEMD_LIBS=elogind` and
set `SYSTEMD_LIB_DIR` to the appropriate directory.

When using elogind, the apis needed for `journal` and `bus` features may not be completely
avaliable (elogind forked from an older version of systemd that may lack some
of these APIs). If your application does not need these features, depend on
`systemd` without the default features to allow maximum compatibility:

```toml
[dependencies]
systemd = { version = "0.5", default-features = false }
```

Note that there still may be some missing symbols. If you discover a link
error, report it so that we can tweak the `systemd` crate to support it.

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
   journal::JournalLog::init().unwrap();
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

