# LSB Dynamic Loader

Runtime `dlopen()` wrappers for LSB (Linux Standard Base) system libraries — currently zlib and OpenSSL/LibreSSL. Enables detection and loading of whichever version is installed on the host, with trust checks and explicit path overrides.

## Crates

| Crate | Description |
|-------|-------------|
| `lsb-loader` | Core dynamic loader: soname candidate search, trust verification (root-owned, non-world-writable, whitelisted paths), env-var path overrides |
| `lsb-zlib` | zlib `compress`/`uncompress` wrapper |
| `lsb-openssl` | OpenSSL 1.0/1.1/3.0 and LibreSSL TLS client/server wrapper |
| `lsb-squeeze-test` | Integration test against Debian Squeeze's ancient libssl 0.9.8 and libz (built in a chroot) |
| `lsb-nix-test` | Integration test against Nix store OpenSSL 3.x and zlib (when Nix is installed) |

## Usage

```rust
use lsb_openssl::Openssl;
let ssl = Openssl::load()?;
ssl.init()?;
let ctx = ssl.ctx_new(true)?; // client context
```

The wrappers search for libraries by soname (`libssl.so.3`, `libssl.so.1.1`, …) and verify trust before loading. To force a specific library path:

```sh
LSBWRAP_LIBSSL_PATH=/path/to/libssl.so \
LSBWRAP_LIBCRYPTO_PATH=/path/to/libcrypto.so \
LSBWRAP_LIBZ_PATH=/path/to/libz.so \
cargo run --example tls_pair
```

## Running tests

```sh
cargo test --workspace
```

The squeeze test builds a Debian Squeeze chroot to verify old-library compatibility. It runs automatically when the chroot exists; enable with:

```sh
SQUEEZE_CHROOT_READY=1 cargo test -- squeeze
```

The Nix test runs when the Nix package manager is installed and nixpkgs is configured:

```sh
nix-channel --add https://nixos.org/channels/nixpkgs-unstable nixpkgs
nix-channel --update
cargo test -p lsb-nix-test
```

## License

MIT
