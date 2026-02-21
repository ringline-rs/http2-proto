# CLAUDE.md

## Build & Test Commands

```bash
# Build
cargo build
cargo build --all-features

# Lint
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --all-features -- -D warnings

# Test
cargo test --all
cargo test --all --all-features
cargo test --all --release

# Docs
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
```

## Architecture

Pure HTTP/2 protocol implementation with optional TLS transport. No runtime dependencies.

### Features

- `tls` (default) — enables TLS transport via `rustls` and `webpki-roots`

### Key Types

- `Frame` — HTTP/2 frame enum (Data, Headers, Settings, WindowUpdate, etc.)
- `FrameDecoder` / `FrameEncoder` — frame parsing and serialization
- `HpackDecoder` / `HpackEncoder` — HPACK header compression
- `Connection` — client-side HTTP/2 connection state machine
- `ServerConnection` — server-side HTTP/2 connection state machine
- `Transport` — transport layer trait (plain TCP or TLS)
- `PlainTransport` — unencrypted transport
- `TlsTransport` — TLS transport using rustls (requires `tls` feature)
