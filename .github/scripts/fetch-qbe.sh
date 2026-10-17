#!/bin/sh
# Clone QBE into $1, retrying: c9x.me is a single small host and occasionally
# refuses connections from CI runners (git exit code 128).
dest=${1:-/tmp/qbe}
for i in 1 2 3 4 5 6; do
  rm -rf "$dest"
  for url in https://c9x.me/git/qbe.git git://c9x.me/qbe.git; do
    if git clone --depth 1 "$url" "$dest" && [ -f "$dest/Makefile" ]; then exit 0; fi
    rm -rf "$dest"
  done
  echo "qbe clone failed (attempt $i), retrying..." >&2
  sleep $((i * 10))
done
exit 1
