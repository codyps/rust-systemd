# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.11.0](https://github.com/codyps/rust-systemd/compare/systemd-0.10.1...systemd-0.11.0) - 2026-10-28

### Added

- [**breaking**] add contextual crate-owned errors
- support custom journal message formatting
- support named socket-activation file descriptors
- add bus connection APIs and standard name-validation errors
- add checked bus-name constructors and owned storage
- accept borrowed and owned C-string arguments consistently
- add safe sd-bus vtable registration
- support typed D-Bus containers
- replace from Option to impl Into<Option>
- add get_display function

### Fixed

- reject overflowing finite bus and journal timeouts
- gate newer native APIs and check selected library capabilities
- make native error ownership transfer explicitly unsafe
- repair sd-bus callback ownership and message views
- fix : follow try_ prefix convention for result api in journal.rs
- fix : Use iovecs.len().try_into() instead of attempting to manually determine if a cast would overflow.

### Other

- clarify argument context and underlying error diagnostics
- cover convertible checked names at every bus API boundary
- describe vtable member signatures and property traversal
- Merge pull request #364 from codyps/dependabot/cargo/pkg-config-0.3.34
- Merge pull request #363 from codyps/dependabot/cargo/serde-1.0.229
- Merge pull request #362 from codyps/dependabot/cargo/log-0.4.34
- Merge pull request #361 from codyps/dependabot/cargo/memchr-2.8.3
- Merge pull request #360 from codyps/dependabot/cargo/libc-0.2.191
- clarify the remaining D-Bus container decoding TODO
- show valid borrowed message reads beside the lifetime error
- remove sd-bus notes from README
- Bump pkg-config from 0.3.32 to 0.3.34
- Bump serde from 1.0.228 to 1.0.229
- Bump log from 0.4.29 to 0.4.34
- Bump memchr from 2.8.0 to 2.8.3
- Bump libc from 0.2.182 to 0.2.191
- Bump cfg-if from 1.0.4 to 1.0.5
- Initialize copied bus errors and use native fallback string lengths
- Drain pending journal entries and honor the complete wait timeout
- Separate safe daemon calls from unsafe environment clearing
- Handle missing bus message errors without dereferencing null
- automate releases with release-plz trusted publishing
- Accept PID values consistently across login queries and aliases
- Add subject-specific login query aliases
- Fix output storage for session VT queries
- Name the user primary-session lookup explicitly and strengthen tests
- Merge current master into journal API PR
- Preserve absent journal cutoffs and clarify unique enumeration
- Merge latest master to validate combined dependency updates
- Merge master into libc dependency update
- Merge master into memchr dependency update
- Merge pull request #338 from codyps/dependabot/cargo/log-0.4.29
- Merge pull request #337 from codyps/dependabot/cargo/doc-comment-0.3.4
- Merge pull request #335 from codyps/dependabot/cargo/cfg-if-1.0.4
- return Result directly from submission APIs
- Added FFI declaration for `sd_journal_has_runtime_files` and exposed it as `has_runtime_files` in the public `JournalRef` API.
- Added FFI binding for `sd_journal_has_persistent_files` and exposed it in the high-level `Journal` API.
- Exposed `sd_journal_get_cutoff_realtime_usec` and `sd_journal_get_cutoff_monotonic_usec` in the high-level `JournalRef` API.
- Added usage method to public api
- Added query unique methods from existing c api to rust api
- Bump libc from 0.2.174 to 0.2.182
- Bump memchr from 2.7.5 to 2.8.0
- Bump log from 0.4.27 to 0.4.29
- Bump doc-comment from 0.3.3 to 0.3.4
- Bump cfg-if from 1.0.1 to 1.0.4
- Add Result-based API for better error handling
- Add rust wrappers for systemd-logind session functions
