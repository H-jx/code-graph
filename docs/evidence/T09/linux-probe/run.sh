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
evidence=docs/evidence/T09/linux-probe
cargo test -p project-knowledge-core --test lifecycle --locked -j 1 > "$evidence/lifecycle.log" 2>&1
cargo test -p project-knowledge-core --test collectors --locked -j 1 > "$evidence/collectors.log" 2>&1
cargo build -p project-knowledge-cli --no-default-features --locked -j 1 > "$evidence/cli-build.log" 2>&1
export PROJECT_KNOWLEDGE_CLI="$CARGO_TARGET_DIR/debug/project-knowledge"
node scripts/verify-dependencies.mjs --output "$evidence/dependencies" > "$evidence/dependencies.log" 2>&1
unshare --mount --propagation private sh "$evidence/full-filesystem.sh"
