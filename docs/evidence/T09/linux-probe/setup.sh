#!/bin/sh
# 本机 WSL 的验收工具，固定版本；不修改业务项目或 shell profile。
set -eu
toolroot=/root/.cache/project-knowledge-linux-probe
mkdir -p "$toolroot/downloads"
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y --no-install-recommends build-essential pkg-config ca-certificates xz-utils python3.12-venv
export RUSTUP_HOME="$toolroot/rustup"
export CARGO_HOME="$toolroot/cargo"
if [ ! -x "$CARGO_HOME/bin/cargo" ]; then
  curl --fail --location --retry 2 --max-time 120 -o "$toolroot/downloads/rustup-init" https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init
  curl --fail --location --retry 2 --max-time 60 -o "$toolroot/downloads/rustup-init.sha256" https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init.sha256
  (cd "$toolroot/downloads" && sha256sum -c rustup-init.sha256)
  chmod u+x "$toolroot/downloads/rustup-init"
  "$toolroot/downloads/rustup-init" -y --no-modify-path --profile minimal --default-toolchain 1.97.1
fi
if [ ! -x "$toolroot/node-v24.14.0-linux-x64/bin/node" ]; then
  curl --fail --location --retry 2 --max-time 120 -o "$toolroot/downloads/node-v24.14.0-linux-x64.tar.xz" https://nodejs.org/dist/v24.14.0/node-v24.14.0-linux-x64.tar.xz
  curl --fail --location --retry 2 --max-time 60 -o "$toolroot/downloads/SHASUMS256.txt" https://nodejs.org/dist/v24.14.0/SHASUMS256.txt
  (cd "$toolroot/downloads" && awk '$2 == "node-v24.14.0-linux-x64.tar.xz"' SHASUMS256.txt | sha256sum -c -)
  tar -xJf "$toolroot/downloads/node-v24.14.0-linux-x64.tar.xz" -C "$toolroot"
fi
export PATH="$CARGO_HOME/bin:$toolroot/node-v24.14.0-linux-x64/bin:$PATH"
rustup run 1.97.1 cargo --version
node --version
python3 --version
gcc --version | head -1
