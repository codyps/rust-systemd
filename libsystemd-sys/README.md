# libsystemd-sys

Raw Rust FFI bindings to libsystemd and compatible libelogind APIs. Functions
and types follow the native `sd_*` API names. Native operations require `unsafe`
calls and return their original C values, including negative errno codes.

For an interface that manages Rust lifetimes, ownership, and error conversion,
use the [`systemd` crate](https://crates.io/crates/systemd).

[API documentation](https://docs.rs/libsystemd-sys) ·
[Native API reference](https://www.freedesktop.org/software/systemd/man/latest/libsystemd.html) ·
[Changelog](https://github.com/codyps/rust-systemd/blob/main/libsystemd-sys/CHANGELOG.md)

## Installation

```toml
[dependencies]
libsystemd-sys = "0.10.0"
```

Install your platform's libsystemd development package and pkg-config before
building. Default features are `bus` and `journal`; they require the capabilities
of systemd 239. To use the base APIs without those bindings:

```toml
[dependencies]
libsystemd-sys = { version = "0.10.0", default-features = false }
```

Base APIs require systemd 233 or equivalent capabilities from libelogind.

## Parse a systemd ID

Provide valid storage and check the native return code before using the output:

```rust
use libsystemd_sys::id128::{sd_id128_from_string, sd_id128_t};
use std::{ffi::CString, io};

fn main() -> io::Result<()> {
    let input = CString::new("0123456789abcdef0123456789abcdef")?;
    let mut id = sd_id128_t { bytes: [0; 16] };
    // SAFETY: input is a live NUL-terminated string and id is writable output.
    let result = unsafe { sd_id128_from_string(input.as_ptr(), &mut id) };
    if result < 0 {
        return Err(io::Error::from_raw_os_error(-result));
    }
    assert_eq!(id.bytes[0], 0x01);
    Ok(())
}
```

Pointer validity, callback lifetimes, thread restrictions, and ownership follow
the native API's contracts. Rust declarations alone do not enforce them. Some
native functions return pointers or use other success conventions; consult the
individual native function's documentation.

## Native API features

Each version feature includes all earlier version features:

| Feature | Additional bindings |
| --- | --- |
| `bus` (default) | D-Bus connections, messages, vtables, and callbacks |
| `journal` (default) | Journal submission and reading |
| `systemd_v240` | Boot-specific application IDs; bus timeout and close-on-exit APIs |
| `systemd_v241` | Bus close-and-unref helper |
| `systemd_v245` | Journal namespaces |
| `systemd_v254` | Session start-time queries |

In 0.10, newer bindings that were previously exposed unconditionally require
their version features. The vtable representation also no longer implements
`Send` or `Sync`. See the changelog for migration details.

## Native library selection

By default, pkg-config locates `libsystemd`. Set `SYSTEMD_PKG_NAME` to use another
pkg-config package, such as `libelogind`. Disable `bus` and `journal` when the
selected library does not supply those APIs.

Set `SYSTEMD_LIB_DIR` to select a library directory directly. Optionally, set
`SYSTEMD_LIBS` to colon-separated names, including a linking kind when needed:
`static=systemd` or `elogind`, for example. Explicit paths do not require
pkg-config or native headers.

The build checks the selected library's actual symbols, including for explicit
library paths. Missing APIs produce an error naming the required capabilities.
Cross compilation uses Cargo's configured target linker and does not execute
target programs.

## License

LGPL-2.1-or-later WITH GCC-exception-2.0. See
[COPYING](https://github.com/codyps/rust-systemd/blob/main/COPYING) for the
license and linking exception.
