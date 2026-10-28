# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.5](https://github.com/codyps/rust-systemd/compare/libsystemd-sys-0.9.4...libsystemd-sys-0.9.5) - 2026-10-28

### Added

- support named socket-activation file descriptors
- add safe sd-bus vtable registration

### Fixed

- gate newer native APIs and check selected library capabilities
- make native error ownership transfer explicitly unsafe
- preserve pointer types in native vtable unions

### Other

- Encode the bus vtable type in the high byte on big-endian targets
- Keep crate release documentation independent
- automate releases with release-plz trusted publishing
- Merge current master into journal API PR
- Merge master into libc dependency update
- Added FFI declaration for `sd_journal_has_runtime_files` and exposed it as `has_runtime_files` in the public `JournalRef` API.
- Added FFI binding for `sd_journal_has_persistent_files` and exposed it in the high-level `Journal` API.
- Bump libc from 0.2.174 to 0.2.182
- Bump cfg-if from 1.0.1 to 1.0.4
- Add rust wrappers for systemd-logind session functions
