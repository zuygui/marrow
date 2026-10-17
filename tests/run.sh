#!/bin/sh
# End-to-end tests: every tests/run/*.mw is compiled with --nostdlib, executed,
# and its standard output compared with the neighbouring .stdout file.
#   tests/run.sh [name...]      MARROW=path/to/marrow to override the compiler
set -u
root=$(cd "$(dirname "$0")/.." && pwd)
marrow=${MARROW:-$root/build/marrow}
exe=""; case "$(uname -s)" in MINGW*|MSYS*) exe=.exe ;; esac
work=$root/build/e2e
rm -rf "$work"; mkdir -p "$work"; cp -r "$root/std" "$work/std"
names=$*; [ -n "$names" ] || names=$(cd "$root/tests/run" && ls *.mw | sed 's/\.mw$//')
limit=""; command -v timeout >/dev/null 2>&1 && limit="timeout 60"
fail=0
for n in $names; do
    cp "$root/tests/run/$n.mw" "$work/$n.mw"
    if ! (cd "$work" && "$marrow" --nostdlib --color never "$n.mw" >/dev/null 2>"$n.log"); then
        echo "FAIL $n: compilation failed"; cat "$work/$n.log"; fail=1; continue
    fi
    if ! (cd "$work" && $limit "./$n$exe" >"$n.out" 2>"$n.log"); then
        echo "FAIL $n: non-zero exit status"; cat "$work/$n.log"; fail=1; continue
    fi
    # line endings are not part of the contract (Windows checkouts may convert them)
    tr -d '\r' < "$work/$n.out" > "$work/$n.out.lf"; tr -d '\r' < "$root/tests/run/$n.stdout" > "$work/$n.exp.lf"
    if cmp -s "$work/$n.out.lf" "$work/$n.exp.lf"; then echo "ok   $n"
    else echo "FAIL $n: output differs"; diff "$work/$n.exp.lf" "$work/$n.out.lf" | head -20; fail=1; fi
done
exit $fail
