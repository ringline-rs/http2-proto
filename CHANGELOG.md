# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.1] - 2026-02-21

### Added

- Initial release extracted from crucible workspace
- Full HTTP/2 frame encoding and decoding
- HPACK header compression
- Connection and stream state management
- Flow control
- Transport layer abstraction (plain TCP, TLS)
- Optional TLS transport via `tls` feature flag (enabled by default)
