#!/bin/sh
# CI runner: test the LSB dynamic loader workspace across distros.
# Usage: ./ci/run_tests.sh [distro_name]
# If distro_name is omitted, runs on the current host.

set -e

if [ -n "$1" ]; then
    DISTRO="$1"
    echo "==> Running CI for distro: $DISTRO"
    case "$DISTRO" in
        ubuntu-20.04|ubuntu-22.04|debian-11|debian-12)
            docker run --rm -v "$PWD:/workspace" -w /workspace \
                "$DISTRO" sh -c "
                    apt-get update -qq && apt-get install -y -qq build-essential curl
                    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
                    . \$HOME/.cargo/env
                    cargo build --release
                    cargo test
                "
            ;;
        rocky-8|rocky-9)
            docker run --rm -v "$PWD:/workspace" -w /workspace \
                "$DISTRO" sh -c "
                    dnf install -y -q gcc curl
                    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
                    . \$HOME/.cargo/env
                    cargo build --release
                    cargo test
                "
            ;;
        alpine-latest)
            docker run --rm -v "$PWD:/workspace" -w /workspace \
                "alpine:latest" sh -c "
                    apk add --no-cache gcc musl-dev curl
                    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
                    . \$HOME/.cargo/env
                    cargo build --release
                    cargo test
                "
            ;;
        *)
            echo "Unknown distro: $DISTRO"
            exit 1
            ;;
    esac
else
    echo "==> Running CI on current host"
    cargo build --release
    cargo test
fi
