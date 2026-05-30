# 1. rustls - 2 versions (CRITICAL but unavoidable)
rustls 0.23.32: Used by hickory-resolver, tokio-rustls, ureq
rustls 0.24.0-dev.0: Used by easyp-server, rustls-acme, acme-lib
Status: Unavoidable - These are different major versions with breaking API changes
Impact: High binary size, but necessary for compatibility
# 2. rand ecosystem - 2 versions (HIGH but unavoidable)
rand 0.8.5: Used by acme-lib, rsa
rand 0.9.2: Used by hickory-resolver, hickory-proto
Status: Unavoidable - Different major versions with breaking changes
Impact: Medium binary size, but necessary for compatibility
# 3. webpki-roots - 2 versions (MEDIUM but unavoidable)
webpki-roots 0.26.11: Used by hickory-resolver
webpki-roots 1.0.2: Used by ureq
Status: Unavoidable - Different major versions
Impact: Low binary size
