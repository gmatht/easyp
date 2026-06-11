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

## Design decisions

### Why `dlsym` instead of `dlvsym`

All symbol resolution in this loader uses `dlsym` (via `libloading::Library::get`), never `dlvsym`. Symbol versioning in ELF (`symname@VERSION`) is intentionally avoided for several reasons:

- **`dlvsym` doesn't solve the real compatibility problem.** The breaks between OpenSSL 1.1 and 3.x aren't about subtle semantic changes to a versioned symbol — they involve removed symbols, renamed functions, and changed struct layouts. `dlvsym` can't conjure a symbol that doesn't exist in the loaded library, nor can it fix ABI mismatches from struct changes. These require explicit fallback logic, which the wrappers already provide (e.g. `TLS_client_method` → `SSLv23_client_method`, `X509_getm_notBefore` → `X509_get_notBefore`, `OPENSSL_sk_new_null` → `sk_X509_EXTENSION_new_null`).

- **Version tags are not a stable interface.** The same upstream version may carry different soname and symbol-version tags across distributions. Hardcoding versioned symbol names would be fragile and platform-dependent.

- **`dlsym` is adaptive.** It returns the default version of a symbol — the one the library considers canonical for its current major version. Pinning to an old version tag via `dlvsym` would select a backward-compatibility shim at best, or a missing symbol at worst.

- **`libloading` has no `dlvsym` support.** Using it would require raw FFI bindings and a parallel resolution path, adding complexity to every call site for no concrete benefit.

We remain open to reconsidering if a future scenario arises where a library ships backward-incompatible behavior under the same symbol name *without* changing the version tag, and where a compat shim bearing the old tag is still available.

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
