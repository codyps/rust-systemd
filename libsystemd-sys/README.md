# libsystemd-sys

Raw FFI bindings to APIs provided by libsystemd and libelogind.
For a safe Rust interface, use the [systemd crate](https://crates.io/crates/systemd).

```toml
[dependencies]
libsystemd-sys = "0.9.5"
```

## Build configuration

By default, the build script uses `pkg-config` to locate `libsystemd`.
Set `SYSTEMD_PKG_NAME` to select a different pkg-config package.

To specify the library directory directly, set `SYSTEMD_LIB_DIR`. Optionally,
set `SYSTEMD_LIBS` to a colon-separated list of library names, including a
`KIND` prefix when needed, such as `static=systemd`.

When linking against libelogind, set `SYSTEMD_PKG_NAME=libelogind`, or set both
`SYSTEMD_LIB_DIR` and `SYSTEMD_LIBS=elogind`. Disable unsupported bus and journal
bindings when they are not needed:

```toml
[dependencies]
libsystemd-sys = { version = "0.9.5", default-features = false }
```

The default features are `bus` and `journal`. Base APIs require the capabilities
of systemd 233; `bus` requires the slot-lifetime APIs from systemd 239.

Additional APIs require explicit version features:

| Feature | Additional bindings |
| --- | --- |
| `systemd_v240` | Boot-specific application IDs, bus timeout and close-on-exit APIs |
| `systemd_v241` | Bus close-and-unref helper |
| `systemd_v245` | Journal namespaces |
| `systemd_v254` | Session start-time queries |

Each version feature includes all earlier version features. The safe `systemd`
crate forwards the same features. Disabling its default features also disables
the corresponding native bus and journal requirements.

The build script links a small target program to check required symbols before
compiling the bindings. It never runs that program, so the check also supports
cross compilation using Cargo's configured target linker. It checks the selected
library's actual symbols rather than assuming libelogind version numbers imply
systemd compatibility. Explicit library paths work without pkg-config or headers
and undergo the same check, including explicit static-library selection. Missing
APIs produce a build-time error identifying the selected features and missing
symbols, rather than a later application link error.

## Releases

See the [release workflow and trusted publishing setup](https://github.com/codyps/rust-systemd/blob/master/.github/RELEASING.md).

## License

LGPL-2.1-or-later with the GCC runtime library exception, as specified in
`Cargo.toml` and `COPYING`.
