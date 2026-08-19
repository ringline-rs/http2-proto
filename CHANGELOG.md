# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.2] - 2026-08-19

### Fixed

- Flow-control windows no longer overflow on peer-controlled WINDOW_UPDATE
  frames. `Stream::increase_send_window` added the increment unchecked: three
  well-formed increments of 2^30 panicked a debug build and wrapped the window
  to a negative value in release, stalling the stream.
  `FlowControl::increase_window` saturated instead, which was memory-safe but
  hid the condition. Both now reject an increment that would take the window
  above 2^31 - 1 and leave it unchanged, and the connection reports it as a
  protocol error, per RFC 7540 section 6.9.1.
- HPACK integer decoding no longer overflows on 32-bit targets. The
  continuation-byte loop computed `(byte & 0x7f) << shift` in `usize`, and at
  the maximum shift of 28 that does not fit in a 32-bit `usize`: debug builds
  panicked and release builds wrapped, on peer-controlled input. The
  contribution is now computed in `u64` and range-checked. 64-bit behavior is
  unchanged and pinned by a test.

### Changed

- `Stream::increase_send_window` and `FlowControl::increase_window` return
  `bool` (`#[must_use]`) instead of `()`, reporting whether the increment was
  applied.

## [0.0.1] - 2026-02-21

### Added

- Initial release extracted from crucible workspace
- Full HTTP/2 frame encoding and decoding
- HPACK header compression
- Connection and stream state management
- Flow control
- Transport layer abstraction (plain TCP, TLS)
- Optional TLS transport via `tls` feature flag (enabled by default)
