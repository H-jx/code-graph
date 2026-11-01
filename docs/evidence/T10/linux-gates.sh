#!/bin/sh
set -eu
toolroot=/root/.cache/project-knowledge-linux-probe
export RUSTUP_HOME="$toolroot/rustup"
export CARGO_HOME="$toolroot/cargo"
export RUSTUP_TOOLCHAIN=1.97.1
export CARGO_TARGET_DIR="$toolroot/target"
export CARGO_INCREMENTAL=0
export PATH="$CARGO_HOME/bin:$toolroot/node-v24.14.0-linux-x64/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
cd /mnt/e/Project/my-project/code-graph
test -s "$RUSTUP_HOME/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin/cargo-clippy"
test -s "$RUSTUP_HOME/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin/clippy-driver"
cargo clippy --version > docs/evidence/T10/linux-clippy-version.log 2>&1
test -s docs/evidence/T10/linux-clippy-version.log
cargo clippy -p project-knowledge-core --all-targets --locked -j 1 -- -D warnings > docs/evidence/T10/linux-clippy-verified.log 2>&1
cargo test -p project-knowledge-core --test lifecycle --locked -j 1 -- incremental_ --test-threads=1 > docs/evidence/T10/linux-incremental-final.log 2>&1
printf '{"exitCode":0,"checks":["clippy-core-all-targets","incremental-final"],"sourceHash":"%s"}\n' "$(sha256sum crates/core/src/index/incremental.rs | cut -d ' ' -f 1)" > docs/evidence/T10/linux-gates-result.json
python3 docs/evidence/T10/release-probe-cache.py > docs/evidence/T10/linux-cache-release.json
