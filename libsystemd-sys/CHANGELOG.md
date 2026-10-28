# Changelog

## [Unreleased]

## [0.10.0](https://github.com/codyps/rust-systemd/compare/libsystemd-sys-0.9.4...libsystemd-sys-0.10.0) - 2026-10-28

### Breaking changes and migration

- Previously unconditional bindings now require explicit version features:
  `sd_id128_get_boot_app_specific`, bus close-on-exit functions, and bus
  method-call timeout functions require `systemd_v240`; `sd_bus_close_unref`
  requires `systemd_v241`. Enable the required feature in your dependency.
  Version features are cumulative, including the existing `systemd_v245` feature.
- Builds check actual native capabilities. Base APIs require systemd 233 or
  equivalent symbols; the default `bus` feature requires systemd 239. Disable
  unsupported features or select a native library that supplies the APIs.
- `sd_bus_vtable` stores typed pointer fields in its native union and no longer
  implements `Send` or `Sync`. Code that moved or shared these descriptors across
  threads must account explicitly for pointer lifetimes and native thread-safety
  requirements. This change and the feature gates require the 0.10 version bump.

### Added

- `sd_listen_fds_with_names` for named socket-activation descriptors. Callers own
  the returned native string vector and must free its allocations appropriately.
- Raw vtable entry constructors for methods, signals, and properties using the
  original v221 layout. Registration remains unsafe and requires valid backing
  strings, callbacks, and userdata throughout its lifetime.
- Journal persistent/runtime-file detection bindings.
- `sd_session_get_start_time` behind `systemd_v254`.
- `systemd_v240`, `systemd_v241`, and `systemd_v254` feature flags.

### Fixed

- Vtable unions preserve pointer types instead of storing pointers as integers.
- Vtable type and flag encoding uses the correct byte layout on big-endian targets.
- Native API checks honor explicit library paths, static-library selection,
  pkg-config backends, and Cargo's target linker. The check never executes target
  programs, including when cross compiling, and reports missing capabilities
  before an application reaches its final link step.
