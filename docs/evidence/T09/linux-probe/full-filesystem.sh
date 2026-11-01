#!/bin/sh
# 仅在独立 mount namespace 中挂载 4 MiB tmpfs，结束时自动释放。
set -eu
owned=$(mktemp -d /tmp/pk-fullfs.XXXXXX)
trap 'umount "$owned" 2>/dev/null || true; rmdir "$owned"' EXIT
mount -t tmpfs -o size=4m,nr_inodes=1024 tmpfs "$owned"
dd if=/dev/zero of="$owned/capacity" bs=4096 count=1024 status=none
export PK_TEST_FULL_FS_ROOT="$owned"
df -B1 "$owned" > docs/evidence/T09/linux-probe/full-filesystem-capacity.log
cargo test -p project-knowledge-core --test lifecycle --locked -j 1 -- --ignored --exact activation_publish_real_filesystem_full_preserves_old_snapshot_and_retries --nocapture > docs/evidence/T09/linux-probe/full-filesystem.log 2>&1
