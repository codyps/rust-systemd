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

The default features are `bus` and `journal`. The `systemd_v245` feature enables
bindings for APIs introduced in systemd 245.

## Releases

See the [release workflow and trusted publishing setup](https://github.com/codyps/rust-systemd/blob/master/.github/RELEASING.md).

## License

LGPL-2.1-or-later with the GCC runtime library exception, as specified in
`Cargo.toml` and `COPYING`.
