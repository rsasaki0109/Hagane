#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release --locked --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/hagane.wasm web/hagane.wasm
