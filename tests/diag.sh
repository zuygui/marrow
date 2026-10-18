#!/bin/sh
# Golden diagnostics: every tests/{ui,typeck,lexer,parser,import,codegen}/*.mw
# is checked (--check --color never) and its stderr compared with the
# neighbouring .stderr file.  BLESS=1 tests/diag.sh regenerates the goldens.
set -u
root=$(cd "$(dirname "$0")/.." && pwd)
marrow=${MARROW:-$root/build/marrow}
fail=0
for d in ui typeck lexer parser import codegen; do
    for f in "$root"/tests/$d/*.mw; do
        n=$(basename "$f")
        # imports like plat_{target}.mw must not depend on the host
        tgt=""; [ "$d" = import ] && tgt="--target x86_64-linux"
        actual=$(cd "$root/tests/$d" && "$marrow" --check --color never $tgt "$n" 2>&1 >/dev/null | tr -d '\r'; echo x)
        golden=${f%.mw}.stderr
        if [ -n "${BLESS:-}" ]; then printf '%s' "${actual%x}" > "$golden"; continue; fi
        expected=$( { cat "$golden" 2>/dev/null | tr -d '\r'; echo x; } )
        if [ "$expected" != "$actual" ]; then echo "FAIL $d/$n"; printf '%s' "${actual%x}" | diff "$golden" - | head -15; fail=1; fi
    done
done
[ $fail = 0 ] && [ -z "${BLESS:-}" ] && echo "diagnostics ok"
exit $fail
