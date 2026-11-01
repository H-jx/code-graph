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
cargo test -p project-knowledge-core --test lifecycle --locked -j 1 -- --test-threads=1 > docs/evidence/T09/linux-probe/rechecked-lifecycle.log 2>&1
unshare -m sh docs/evidence/T09/linux-probe/full-filesystem.sh
