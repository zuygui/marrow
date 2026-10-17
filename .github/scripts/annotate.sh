#!/bin/sh
# annotate.sh LABEL CMD...: run CMD; on failure, publish the interesting lines
# of its output as a workflow annotation (raw job logs are not always reachable).
label=$1; shift
out=$(mktemp)
"$@" >"$out" 2>&1; rc=$?
cat "$out"
if [ $rc -ne 0 ]; then
  msg=$(grep -E 'FAIL|error|Error|panicked|differs|undefined|ld:|clang' "$out" | head -n 10 | tr '\n' '|' | cut -c1-900)
  msg="$msg ## $(tail -n 14 "$out" | tr '\n' '|' | cut -c1-1200)"
  echo "::error::$label failed ($rc): $msg"
  head -c 2400 "$out" | tr "\n" "|" | fold -w 800 | while IFS= read -r l; do echo "::warning::head: $l"; done
fi
exit $rc
