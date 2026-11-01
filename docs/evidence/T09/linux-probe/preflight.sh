#!/bin/sh
set -eu
owned=$(mktemp -d /tmp/pk-full-preflight.XXXXXX)
case "$owned" in /tmp/pk-full-preflight.*) ;; *) exit 2;; esac
trap 'umount "$owned" 2>/dev/null || true; rmdir "$owned"' EXIT
mount -t tmpfs -o size=4m tmpfs "$owned"
dd if=/dev/zero of="$owned/capacity" bs=4096 count=1024 status=none
df -B1 "$owned"
if printf x > "$owned/probe"; then exit 2; else exit 0; fi
