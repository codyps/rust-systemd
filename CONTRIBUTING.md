# Contributing

The workspace contains the safe `systemd` crate and the raw `libsystemd-sys`
bindings. Crate READMEs are also published on crates.io, so keep them focused on
installation, usage, examples, and API documentation.

Install a native libsystemd development package and pkg-config before building.
The default features require the capabilities of systemd 239. Tests using newer
`systemd_v*` features require a native library that supplies those APIs.

Run formatting, Clippy, and the workspace tests before submitting a change:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The integration tests need the native APIs and D-Bus services they exercise.
CI checks the supported feature combinations and native linking behavior.

See [.github/RELEASING.md](.github/RELEASING.md) for release preparation,
trusted publishing, and automation setup.

## License

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be licensed as described in
[COPYING](COPYING), without any additional terms or conditions.
