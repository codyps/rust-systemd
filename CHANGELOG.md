# Changelog

## [Unreleased]

## [0.11.1](https://github.com/codyps/rust-systemd/compare/systemd-0.11.0...systemd-0.11.1) - 2026-10-28

### Fixed

- enable Serde allocation for standalone feature builds

## [0.11.0](https://github.com/codyps/rust-systemd/compare/systemd-0.10.1...systemd-0.11.0) - 2026-10-28

### Breaking changes and migration

- `systemd::Error` is now a crate-owned type instead of a re-export of
  `std::io::Error`, and `systemd::Result` uses it. It preserves error kinds and
  errno while adding operation and argument context. `?` converts between the
  two error types; direct `io::Result` assignments or returns need conversion,
  such as `.map_err(Into::into)`, or a `systemd::Result` signature. User-defined
  trait implementations and callbacks using `systemd::Result` must use the new
  error type. Converting native errors back to `io::Error` preserves errno and
  drops added context; `into_io_error_with_context()` retains context instead.
- Journal `send`, `print`, `log`, `log_record`, and `sd_journal_log!` now return
  `Result<()>`. Handle errors with `?` or explicitly, rather than checking integer
  return codes or assuming an infallible call. These are the primary function
  names; no separate `try_*` variants are needed. `Ok(())` does not guarantee
  message delivery. The `log::Log` adapter remains best effort.
- Safe daemon functions no longer accept an environment-clearing boolean and
  leave the process environment unchanged. Remove that argument from
  `listen_fds`, `notify`, `pid_notify`, `pid_notify_with_fds`, and
  `watchdog_enabled`. Use their explicit unsafe `*_unset_environment` variants
  only when all concurrent environment access can be prevented.
- `ObjectPath`, `BusName`, `InterfaceName`, and `MemberName` `from_bytes`
  constructors return `Result` instead of `Option`. Propagate or handle their
  validation errors. String-taking APIs use `systemd::CStrArgument`, replacing
  the `cstr-argument` dependency; custom argument implementations must implement
  the crate's fallible trait.
- Native bus error ownership transfer through `RawError::from_raw` requires
  `unsafe`. The safe conversion from an unvalidated native error has been removed.
- Native APIs require systemd 233 or equivalent capabilities; the default `bus`
  feature requires systemd 239. Newer APIs require the cumulative `systemd_v240`,
  `systemd_v241`, `systemd_v245`, or `systemd_v254` features. In particular,
  boot-specific application IDs and bus method-call timeout methods require
  `systemd_v240`. Disable unsupported features for older or partial backends.
- PID queries and their aliases accept a PID, `Some(pid)`, or `None`. Both `None`
  and `0` select the current process. Add a type annotation to ambiguous
  `Default::default()` or `.into()` arguments, or pass `None` / `Some(pid)` directly.

### Added

- Generic C-string inputs accepting Rust strings, C strings, owned values, and
  `Cow`, with validation errors and allocation avoidance for suitable inputs.
- Checked bus-name constructors and owned `ObjectPathBuf`, `BusNameBuf`,
  `InterfaceNameBuf`, and `MemberNameBuf` types.
- Safe D-Bus vtables for methods, signals, and readable or writable properties,
  with owned callback storage and native slot cleanup.
- Typed D-Bus arrays, structs, dictionaries, variants, and borrowed values.
- Independent bus connections through `Bus::open`, `open_user`, `open_system`,
  `open_system_remote`, and `open_system_machine`, plus explicit flush and close.
- `JournalLog::with_formatter` for custom message text with structured metadata,
  global installation, and a fallible configured logger's `log_record` method.
- Named socket-activation descriptors through `listen_fds_with_names` and its
  explicit unsafe environment-clearing variant. Names retain non-UTF-8 bytes and
  duplicate matches; dropping the names does not close descriptors.
- Journal usage, persistent/runtime-file detection, cutoff timestamps, and unique
  field queries.
- Login session queries, `user_primary_session`, and process/session aliases.
  Existing C-backed `get_*` names remain supported. Session start-time queries
  require `systemd_v254`.

### Fixed

- Journal field-count overflow and field-buffer lifetimes; submission errors now
  propagate while optional record source locations remain supported.
- Finite journal and bus timeout overflow returns `EOVERFLOW`; `None` remains the
  way to request an infinite wait.
- Journal entry waiting drains available entries and honors the complete timeout
  after file-change notifications. Missing cutoff timestamps return `None`.
- Session VT queries pass correctly sized native output storage.
- D-Bus callback ownership, slot teardown, panic handling, message lifetimes, and
  absent message errors. Native error copying and allocation fallbacks retain
  valid owned storage and correct string lengths.
- Native symbol checks validate the selected library and feature requirements,
  including manual linking, libelogind, static libraries, and cross compilation.
