#!/bin/sh
# annotate.sh LABEL CMD...: run CMD; on failure, publish the interesting lines
# of its output as a workflow annotation (raw job logs are not always reachable).
label=$1; shift
out=$(mktemp)
"$@" >"$out" 2>&1; rc=$?
cat "$out"
if [ $rc -ne 0 ]; then
  msg=$(grep -E 'FAIL|error|Error|panicked|differs|undefined|ld:|clang' "$out" | head -n 10 | tr '\n' '|' | cut -c1-900)
  [ -n "$msg" ] || msg=$(tail -n 8 "$out" | tr '\n' '|' | cut -c1-900)
  echo "::error::$label failed ($rc): $msg"
fi
exit $rc
