# systemd

Rust interfaces to libsystemd for the journal, socket activation, service
notifications, login sessions, IDs, and D-Bus. For direct C bindings, use
[`libsystemd-sys`](https://crates.io/crates/libsystemd-sys).

[API documentation](https://docs.rs/systemd) ·
[Changelog](https://github.com/codyps/rust-systemd/blob/main/CHANGELOG.md)

## Installation

```toml
[dependencies]
systemd = "0.11.2"
```

This crate links to a native libsystemd installation. With the default features,
you need systemd 239 or a library providing equivalent APIs. Install your
platform's libsystemd development package and pkg-config before building.

The `journal` and `bus` features are enabled by default. To use only daemon,
login, unit-name, and ID APIs:

```toml
[dependencies]
systemd = { version = "0.11.2", default-features = false }
```

Without `bus`, the base native requirements are systemd 233 or equivalent.
A compatible libelogind can provide these base APIs; see native library selection
below.

## Write to the journal

Send structured fields, including your own fields, in `NAME=value` form:

```rust
use systemd::journal;

fn main() -> systemd::Result<()> {
    journal::send(&[
        "MESSAGE=Worker started",
        "PRIORITY=6",
        "WORKER=queue-a",
    ])?;
    Ok(())
}
```

Journal submission functions return `systemd::Result<()>`. Success means
libsystemd accepted the submission; it does not guarantee delivery to journald.

To use the `log` facade, also add `log = "0.4"` to your dependencies. You can
customize the message while keeping priority, target, and source locations as
structured journal fields:

```rust
use systemd::journal::JournalLog;

fn main() -> Result<(), log::SetLoggerError> {
    JournalLog::with_formatter(|out, record| {
        write!(out, "[{}] {}", record.target(), record.args())
    }).install()?;
    log::set_max_level(log::LevelFilter::Info);
    log::info!(target: "worker", "ready");
    Ok(())
}
```

## Read named socket-activation descriptors

Services started by systemd can inspect the file descriptors supplied at startup:

```rust
use systemd::daemon;

fn main() -> systemd::Result<()> {
    let descriptors = daemon::listen_fds_with_names()?;
    for fd in descriptors.iter_named("http")? {
        println!("HTTP descriptor: {fd}");
    }
    Ok(())
}
```

The returned descriptor numbers are borrowed process resources; dropping the
metadata does not close them. Safe daemon functions leave the process environment
unchanged. Explicit unsafe `*_unset_environment` variants are available for
single-threaded startup code that can prevent all concurrent environment access.

## Other APIs

- [`journal`](https://docs.rs/systemd/latest/systemd/journal/) reads, filters,
  seeks, and waits for journal entries.
- [`bus`](https://docs.rs/systemd/latest/systemd/bus/) opens local, remote, and
  container connections, sends typed messages, and registers service vtables.
- [`login`](https://docs.rs/systemd/latest/systemd/login/) queries processes,
  sessions, seats, and users.
- [`id128`](https://docs.rs/systemd/latest/systemd/id128/) handles systemd IDs.
- [`unit`](https://docs.rs/systemd/latest/systemd/unit/) escapes unit names.

Fallible systemd operations return the crate-owned
[`Error`](https://docs.rs/systemd/latest/systemd/struct.Error.html), with errno,
I/O error kind, operation, argument context, and underlying cause where available.
Its documentation includes migration from `std::io::Error` and conversion options.

## Native API features

Enable a version feature only when your native library provides its APIs. Each
version feature includes the earlier version features.

| Feature | Additional APIs |
| --- | --- |
| `systemd_v240` | Boot-specific application IDs; bus method-call timeout APIs |
| `systemd_v241` | Raw bus close-and-unref helper |
| `systemd_v245` | Journal namespaces |
| `systemd_v254` | Login session start-time queries |
| `serde` | Serialization and deserialization of `Id128` |

The build checks the selected library's symbols and reports missing capabilities.
See the [API documentation](https://docs.rs/systemd) for feature requirements on
individual functions.

## Native library selection

By default, pkg-config locates `libsystemd`. Set `SYSTEMD_PKG_NAME=libelogind`
to select a compatible libelogind installation, with unsupported `bus` and
`journal` features disabled.

For a library outside pkg-config's search path, set `SYSTEMD_LIB_DIR` to its
library directory. `SYSTEMD_LIBS` selects colon-separated library names, optionally
with a linking kind, such as `SYSTEMD_LIBS=static=systemd`. For libelogind, use
`SYSTEMD_LIBS=elogind` alongside the library directory.

## License

LGPL-2.1-or-later WITH GCC-exception-2.0. See
[COPYING](https://github.com/codyps/rust-systemd/blob/main/COPYING) for the
license and linking exception.
