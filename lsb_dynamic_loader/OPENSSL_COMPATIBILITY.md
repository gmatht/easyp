# OpenSSL / LibreSSL Compatibility

`lsb-openssl` dynamically loads the system's SSL library at runtime.
It is compatible with a wide range of OpenSSL and LibreSSL versions.

## Minimum Requirements

| Library | Minimum Version | Notes |
|---------|----------------|-------|
| **OpenSSL** | 1.1.0 | 1.0.2 is **not supported** (lacks `EVP_DigestSign*` and other 1.1+ APIs) |
| **LibreSSL** | 3.7.0 | All LibreSSL versions that provide `libcrypto.so.3` / `libssl.so.3` |

Attempting to run on an older library produces a clear error message:

```
Error: "other: sym: EVP_DigestSign
  → Your SSL library (OpenSSL10) is too old for this build.
  → OpenSSL ≥1.1 or LibreSSL ≥3.7 required.
  → Install a newer OpenSSL or use a different SSL backend."
```

OpenSSL 1.0.x is not supported because it lacks support for the modern ECDSA encryption algorithm. While RSA's theoretical weakness to quantum computing may not be an immediate concern, centos 7 is no longer supported so there is also no pressing need to support 1.0 either.

## Tested Versions

All tested on the **CentOS 7-built binary** (glibc 2.17) for maximum portability.

### OpenSSL

| Version | Tested On | Status |
|---------|-----------|--------|
| 1.1.1 | AlmaLinux 8 | ✅ |
| 3.0.2 | Ubuntu 22.04 | ✅ |
| 3.0.9 | Fedora 38 | ✅ |
| 3.0.13 | Ubuntu 24.04 / Remote server | ✅ |
| 3.0.20 | Debian 12 | ✅ |
| 3.1.4 | Fedora 39 | ✅ |
| 3.1.8 | Alpine 3.18 / 3.19 | ✅ |
| 3.2.4 | Fedora 40 | ✅ |
| 3.2.6 | Fedora 41 / 42 | ✅ |
| 3.3.7 | Alpine 3.20 / 3.21 | ✅ |
| 3.4.x | Arch Linux (latest) | ✅ |
| 3.5.5 | AlmaLinux 9 | ✅ |

### LibreSSL

| Version | Tested On | Status |
|---------|-----------|--------|
| 3.7.3 | Alpine 3.18 | ✅ |
| 3.8.2 | Alpine 3.19 | ✅ |
| 3.8.4 | Alpine 3.20 | ✅ |
| 4.0.0 | Alpine 3.21 | ✅ |

### NixOS Coverage

All NixOS releases ship with OpenSSL ≥1.1 or LibreSSL:

| NixOS Range | OpenSSL Version | Status |
|-------------|----------------|--------|
| 20.09 – 22.05 | 1.1.1 | ✅ |
| 22.11 – 24.11+ | 3.x | ✅ |

## Implementation Notes

The `lsb-openssl` crate uses `dlopen` / `dlsym` to load SSL symbols at runtime.
API differences between OpenSSL versions are handled with fallback symbol lookups:

- `SSL_CTX_set_tlsext_servername_callback` → falls back to `SSL_CTX_set_cert_cb` (OpenSSL 3.x)
- `X509_getm_notBefore` / `X509_getm_notAfter` → fall back to `X509_get_notBefore` / `X509_get_notAfter` (1.0.x)
- `sk_X509_EXTENSION_new_null` → falls back to `OPENSSL_sk_new_null` (OpenSSL 3.x)
- `i2d_PKCS8PrivateKey_bio` 7-arg form → works on all versions via x86\_64 SysV ABI (extra register args are ignored by older 2-arg implementations)
- Curve NIDs resolved dynamically via `OBJ_txt2nid` instead of hardcoded values
