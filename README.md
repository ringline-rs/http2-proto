# http2-proto

HTTP/2 protocol implementation with optional TLS transport.

A standalone, completion-based HTTP/2 implementation with no async runtime dependencies.

## Features

- Full HTTP/2 frame encoding and decoding
- HPACK header compression
- Connection and stream state management
- Flow control
- Transport layer abstraction (plain TCP, TLS)
- Optional **TLS** transport via the `tls` feature flag (enabled by default)

## Usage

```toml
[dependencies]
http2-proto = "0.0.1"

# Without TLS:
http2-proto = { version = "0.0.1", default-features = false }
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT License](LICENSE-MIT) at your option.
