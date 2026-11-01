#!/bin/sh
set -eu
export RUSTUP_HOME=/root/.cache/project-knowledge-linux-probe/rustup
export CARGO_HOME=/root/.cache/project-knowledge-linux-probe/cargo
export PATH="$CARGO_HOME/bin:/usr/bin:/bin"
# 只移除本任务独立工具链中已证实为零字节的两个损坏文件。
for name in cargo-clippy clippy-driver; do
    path="$RUSTUP_HOME/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin/$name"
    if test -f "$path" && ! test -s "$path"; then rm -- "$path"; fi
done
rustup component add clippy --toolchain 1.97.1
test -s "$RUSTUP_HOME/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin/cargo-clippy"
test -s "$RUSTUP_HOME/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin/clippy-driver"
