#!/bin/sh
# Build (and optionally install) QBE from $1; on failure, surface the tail of
# the output as a workflow annotation so it is readable without the raw logs.
dir=$1; shift
out=$(mktemp)
if ! { make -C "$dir" "$@" && { [ -z "$QBE_INSTALL" ] || $QBE_INSTALL make -C "$dir" install; }; } >"$out" 2>&1; then
  cat "$out"
  echo "::error::qbe build failed: $(tail -n 12 "$out" | tr '\n' '|' | cut -c1-900)"
  exit 1
fi
