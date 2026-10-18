#!/bin/sh
# CLI behaviour: subcommands, argument passing, tool discovery and its errors.
set -u
root=$(cd "$(dirname "$0")/.." && pwd)
marrow=${MARROW:-$root/build/marrow}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0
t() { echo "FAIL $1"; fail=1; }

cp -r "$root/std" "$tmp/std"
cat > "$tmp/h.mw" <<'MW'
@import("std/std.mw")
@export fn main(argc: i32, argv: rawptr) -> i32 {
    env_init(argc, argv);
    for var i: i64 = 1; i < arg_count(); i += 1 { println(arg_get(i)); }
    ret 3;
}
MW
qbe_bin=$(command -v qbe)
cd "$tmp"

"$marrow" version | grep -q '^Marrow version: v' || t version
"$marrow" --version | grep -q '^Marrow version: v' || t --version
"$marrow" help | grep -q 'Commands:' || t help
"$marrow" check h.mw 2>&1 | grep -q 'no errors found' || t check
"$marrow" doctor | grep -q 'qbe: ' || t doctor

out=$("$marrow" run h.mw -- a 'b c' --x 2>/dev/null); rc=$?
[ "$rc" = 3 ] || t "run exit code ($rc)"
[ "$out" = "a
b c
--x" ] || t "run arguments: $out"

"$marrow" run --bad h.mw >/dev/null 2>"$tmp/err"; [ $? = 2 ] || t "bad option status"
grep -q 'after `--`' "$tmp/err" || t "bad option hint"
"$marrow" run >/dev/null 2>&1; [ $? = 2 ] || t "missing input status"

# legacy form still builds
"$marrow" h.mw >/dev/null 2>&1 && [ -x ./h ] || t "legacy build"

# tool discovery: qbe next to the binary, nothing else in PATH
mkdir bin; cp "$marrow" bin/marrow; cp "$(command -v qbe)" bin/qbe
PATH=/usr/bin:/bin ./bin/marrow doctor | grep -q "qbe: *$tmp/bin/qbe" || t "qbe next to marrow"
cp -r std bin/std
(cd / && PATH=/usr/bin:/bin "$tmp/bin/marrow" check "$tmp/h.mw" >/dev/null 2>&1) || t "std next to marrow"

# missing tools give actionable errors
MARROW_QBE=/nonexistent PATH=/nonexistent "$marrow" h.mw 2>&1 | grep -q '`qbe` was not found' || t "qbe missing message"
MARROW_CC=/nonexistent PATH=/nonexistent MARROW_QBE=$qbe_bin "$marrow" h.mw 2>&1 | grep -q 'no C compiler' || t "cc missing message"
[ $fail = 0 ] && echo "cli ok"
exit $fail
