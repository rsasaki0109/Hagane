#!/usr/bin/env bash
# Prepare the existing cloud checkout; do not create a worktree or alter sources.
set -euo pipefail
export CARGO_HOME=/workspace/.hagane-tools/cargo
export RUSTUP_HOME=/workspace/.hagane-tools/rustup
export PATH="$CARGO_HOME/bin:$PATH"
export NPM_CONFIG_CACHE=/workspace/.hagane-tools/npm
mkdir -p /workspace/.hagane-tools
if [[ ! -x "$CARGO_HOME/bin/rustup" ]]; then
  installer=$(mktemp /tmp/hagane-rustup.XXXXXX)
  trap 'rm -f "$installer"' EXIT
  curl --proto '=https' --tlsv1.2 -fsS https://sh.rustup.rs -o "$installer"
  sh "$installer" -y --no-modify-path --profile minimal --default-toolchain none
fi
rustup toolchain install 1.99.0 --profile minimal --component rustfmt,clippy --target wasm32-unknown-unknown
cat > /workspace/.hagane-tools/env.sh <<'ENV'
export CARGO_HOME=/workspace/.hagane-tools/cargo
export RUSTUP_HOME=/workspace/.hagane-tools/rustup
export PATH="$CARGO_HOME/bin:$PATH"
export NPM_CONFIG_CACHE=/workspace/.hagane-tools/npm
ENV
cd /workspace/Hagane
command -v node >/dev/null
command -v npm >/dev/null
command -v python3 >/dev/null
cargo fetch --locked
npm ci --ignore-scripts
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
./scripts/build-web.sh
node scripts/test-wasm.mjs
if [[ -z "${HAGANE_CHROMIUM:-}" ]] && command -v chromium >/dev/null; then
  export HAGANE_CHROMIUM
  HAGANE_CHROMIUM=$(command -v chromium)
fi
npm test
