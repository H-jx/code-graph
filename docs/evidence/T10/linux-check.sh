#!/bin/sh
set -eu
toolroot=/root/.cache/project-knowledge-linux-probe
export RUSTUP_HOME="$toolroot/rustup"
export CARGO_HOME="$toolroot/cargo"
export RUSTUP_TOOLCHAIN=1.97.1
export CARGO_TARGET_DIR="$toolroot/target"
export CARGO_INCREMENTAL=0
export PATH="$CARGO_HOME/bin:$toolroot/node-v24.14.0-linux-x64/bin:$PATH"
cd /mnt/e/Project/my-project/code-graph
cargo test -p project-knowledge-core --tests --locked -j 1 -- --test-threads=1 > docs/evidence/T10/linux-core.log 2>&1
cargo test -p project-knowledge-cli --no-default-features --test collect --locked -j 1 -- --test-threads=1 > docs/evidence/T10/linux-cli.log 2>&1
rustup component add clippy --toolchain 1.97.1 > docs/evidence/T10/linux-clippy-install.log 2>&1
cargo clippy -p project-knowledge-core --all-targets --locked -j 1 -- -D warnings > docs/evidence/T10/linux-clippy.log 2>&1
