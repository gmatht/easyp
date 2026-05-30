---
title: "Downloads"
description: "Download easyp for your platform"
---

# Downloads

Download easyp version 0.1.3 for your platform. All binaries are statically linked and include automatic ACME certificate management.

## Quick Start

### Linux/Mac
```bash
curl https://www.easyp.net/quickstart-0.1.3.sh -o quickstart-0.1.3.sh && [ "$(sha256sum < quickstart-0.1.3.sh)" = "dd193ae54c23030dd70c8039692e41c174b85ce023dd6b542f412d183b3fdd93  -" ] && sudo bash quickstart-0.1.3.sh
```

### Windows x64
Download `easyp-0.1.3-x64.zip`, extract, and run `easyp.exe`. Administrator privileges required for port 80/443.

---

## All Downloads

### Linux

#### Linux x64 (Intel/AMD)
- **Full Version** (with extensions): [easyp-0.1.3-x64.gz](/easyp-0.1.3-x64.gz)  
  Size: ~1.2MB (compressed) | SHA256: `5c53668f055fb8eed4ca9dd1b7bbbb4f8b01fa2d1155f6b116bc9bf78cd66282`

- **Minimal Version** (no extensions): [easyp-0.1.3-x64-minimal.gz](/easyp-0.1.3-x64-minimal.gz)  
  Size: ~1.2M | SHA256: `d492bec9c3a15be9496a2e200ea92b7883b57b1cd0eb0a6735b2bdd09392d2df`

#### Linux ARM64 (Raspberry Pi, AWS Graviton, etc.)
- **Full Version** (with extensions): [easyp-0.1.3-aarch64.gz](/easyp-0.1.3-aarch64.gz)  
  Size: ~1.3M | SHA256: `d59dcad42017b4c2016eb5ec119b96a17502fee37b3cbabbdfdaeac9af5a5283`

- **Minimal Version** (no extensions): [easyp-0.1.3-aarch64-minimal.gz](/easyp-0.1.3-aarch64-minimal.gz)  
  Size: ~1.3M | SHA256: `9fd293c294ecb32aa313d36ee9f9a5bccd3d35f122c8f5e9927f346ac4acb515`

### Windows

#### Windows x64
- **Full Version** (with extensions): [easyp-0.1.3-x64.zip](/easyp-0.1.3-x64.zip)  
  Size: ~1.3M | SHA256: `032f2b53a75e8cebed95bdf0cdd38d6771375866c47c8b3ad2f93cefc47dc416`

- **Minimal Version** (no extensions): [easyp-0.1.3-x64-minimal.zip](/easyp-0.1.3-x64-minimal.zip)  
  Size: ~1.3M | SHA256: `ae76665094660ece95f44bbbb7ea2638f697d5b9588a3101de945c6f5d0fee5f`

### macOS

#### macOS (Apple Silicon / ARM64)
- **Full Version** (with extensions): [easyp-0.1.3-aarch64-darwin.gz](/easyp-0.1.3-aarch64-darwin.gz)  
  Size: ~1.3M | SHA256: `b44dde7437547361da3bc3013206c82561003e97936f63e433762c958d4f61c3`

- **Minimal Version** (no extensions): [easyp-0.1.3-aarch64-darwin-minimal.gz](/easyp-0.1.3-aarch64-darwin-minimal.gz)  
  Size: ~1.3M | SHA256: `48913d14c24b76a5ee09f8eafea2a791fad95f76a06af777f04bc59379df560c`

#### macOS (Intel)
- **Full Version** (with extensions): [easyp-0.1.3-x86_64-darwin.gz](/easyp-0.1.3-x86_64-darwin.gz)  
  Size: ~1.3M | SHA256: `ebdd4357b51cba0e1241d7a9bcb1c6364e88cb9263ebfcda452d6c7b3196ca50`

- **Minimal Version** (no extensions): [easyp-0.1.3-x86_64-darwin-minimal.gz](/easyp-0.1.3-x86_64-darwin-minimal.gz)  
  Size: ~1.3M | SHA256: `7896754dceaf4025d15c6cc6fcadf5658fd2020b926412e11fa6900effac2d0f`

### Source Code

- **Source Tarball**: [easyp-0.1.3.tgz](/easyp-0.1.3.tgz)  
  Size: ~33M | SHA256: `5116aa2bdd5b8811d9e9be97d50b9714eca4342c262e31857aa07642ced69468`

---

## Full vs Minimal Builds

### Full Version (Default)
Includes all extensions for maximum functionality:
- Admin panel system
- Comment system
- File upload management
- Math and hash template expansions
- Worm append-only storage
- Stats and logging

Use the full version if you need admin features or any of the built-in extensions.

### Minimal Version
Stripped-down version with only core functionality:
- Basic HTTPS server
- ACME certificate management
- Secure file serving
- No admin panel or extensions

Use the minimal version for maximum performance on resource-constrained systems or when you don't need admin features.

---

## Building from Source

### Prerequisites
- Rust 1.83+ (install from [rustup.rs](https://rustup.rs))
- Cross-compilation tools (optional, for cross-platform builds)

### Build Commands

**Full version with all extensions:**
```bash
git clone https://github.com/gmatht/easyp.git
cd easyp
./clonea  # Clone required repositories
cargo build --release --bin easyp
```

**Minimal version without extensions:**
```bash
cargo build --release --bin easyp --no-default-features --features acme,crypto-ring
```

**Cross-compilation (example for Linux from Windows):**
```bash
cross build --release --target x86_64-unknown-linux-gnu --bin easyp
```

See the [development documentation](/content/development) for more details.

---

## Verifying Downloads

### Checksum Files
Download the official checksum files for automated verification:
- **SHA256**: [SHA256SUM](/SHA256SUM)
- **SHA512**: [SHA512SUM](/SHA512SUM)

### Linux/macOS
```bash
# Download and verify using checksum file
curl -O https://www.easyp.net/SHA256SUM
sha256sum -c SHA256SUM

# Or verify individual files
sha256sum easyp-0.1.3-x64.gz
# Compare with SHA256 shown above
```

### Windows (PowerShell)
```powershell
# Download and verify using checksum file
Invoke-WebRequest -Uri "https://www.easyp.net/SHA256SUM" -OutFile "SHA256SUM"
Get-FileHash easyp-0.1.3-x64.zip -Algorithm SHA256
# Compare with SHA256 shown above
```

---

## Previous Releases

Older releases and development versions are available at `/easyp-<version>-*` on this site.

**Latest stable**: 0.1.3  
**Previous stable**: 0.1.2

See the [release notes](/content/release) for details about each version.
