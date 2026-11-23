#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if ! command -v cargo >/dev/null 2>&1; then
    source "$HOME/.cargo/env"
fi
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-6}"
export CARGO_PROFILE_RELEASE_DEBUG=0
export CARGO_PROFILE_RELEASE_LTO=thin
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=8
cargo build --locked --release -p rioterm --no-default-features --features wayland,wgpu
cargo test --locked --release -p rioterm --no-default-features --features wayland,wgpu renderer::island::tests
cargo test --locked --release -p rioterm -p rio-backend --no-default-features --features rioterm/wayland,rioterm/wgpu config::navigation
