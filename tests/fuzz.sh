#!/bin/sh
# Mutation fuzzing of the front end (see tests/fuzz/fuzz.mw).
#   tests/fuzz.sh [iterations] [seed]
set -u
root=$(cd "$(dirname "$0")/.." && pwd)
marrow=${MARROW:-$root/build/marrow}
exe=""; case "$(uname -s)" in MINGW*|MSYS*) exe=.exe ;; esac
work=$root/build/fuzz
rm -rf "$work"; mkdir -p "$work"; cp -r "$root/std" "$work/std"; cp -r "$root/compiler" "$work/compiler"
cp "$root/tests/fuzz/fuzz.mw" "$work/fuzz.mw"
(cd "$work" && "$marrow" --nostdlib --color never fuzz.mw >/dev/null 2>build.log) || { echo "FAIL: cannot build the fuzzer"; cat "$work/build.log"; exit 1; }
cd "$work"
./fuzz$exe "$marrow" "$root/tests/run" "$root/tests/typeck" "$root/tests/ui" "$root/tests/parser" "$root/tests/lexer" "$root/tests/codegen" -n "${1:-300}" -s "${2:-1}"
