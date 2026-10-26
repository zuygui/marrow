#!/bin/sh
# CLI behaviour: subcommands, argument passing, tool discovery and its errors.
set -u
root=$(cd "$(dirname "$0")/.." && pwd)
marrow=${MARROW:-$root/build/marrow}
tmp=$(cd "$(mktemp -d)" && pwd -P)
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
"$marrow" help | grep -q 'Projects and packages:' || t help
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

# marrow fmt
mkdir fmtp; printf 'fn main()->i32{\n  var x:i32=1+2;   // keep\n\n\n\n  if x>2{ret 0;}\n  ret 1;}\n' > fmtp/a.mw
"$marrow" fmt --check fmtp >/dev/null 2>&1; [ $? = 1 ] || t "fmt --check status"
"$marrow" fmt fmtp >/dev/null 2>&1 || t "fmt status"
grep -q '^    var x: i32 = 1 + 2;   // keep$' fmtp/a.mw || t "fmt output: $(cat fmtp/a.mw)"
[ "$(grep -c '^$' fmtp/a.mw)" = 1 ] || t "fmt blank lines"
"$marrow" fmt --check fmtp >/dev/null 2>&1 || t "fmt idempotent"
printf 'fn main() { "abc\n' > fmtp/bad.mw
"$marrow" fmt fmtp >/dev/null 2>&1; [ $? = 1 ] || t "fmt unparsable status"
rm fmtp/bad.mw
"$marrow" fmt --check "$root/std" "$root/compiler" >/dev/null 2>&1 || t "std and compiler are formatted"

# diagnostics as JSON, and the language server
printf 'fn main() -> i32 { var x: i32 = "a"; ret 0; }\n' > bad.mw
"$marrow" check --message-format json --color never bad.mw 2>jd.txt >/dev/null; [ $? = 1 ] || t "json diagnostics status"
grep -q '"code":"E0402"' jd.txt && grep -q '"line":1' jd.txt || t "json diagnostics: $(cat jd.txt)"
lsp_msg() { printf 'Content-Length: %s\r\n\r\n%s' "${#1}" "$1"; }
{ lsp_msg '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'; lsp_msg '{"jsonrpc":"2.0","method":"exit"}'; } | "$marrow" lsp > lsp.txt 2>&1
grep -q 'documentFormattingProvider' lsp.txt || t "lsp initialize: $(cat lsp.txt)"
{ lsp_msg '{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file://'"$tmp"'/bad.mw","text":""}}}'; lsp_msg '{"jsonrpc":"2.0","method":"exit"}'; } | "$marrow" lsp > lsp.txt 2>&1
grep -q 'E0402' lsp.txt || t "lsp diagnostics: $(cat lsp.txt)"

# marrow doc
mkdir -p docp/src; printf '[package]\nname = "d"\nversion = "0.1.0"\n' > docp/marrow.toml
printf '//! Demo.\n\n/// Adds.\n@allow(unused)\nfn add(a: i64, b: i64) -> i64 { ret a + b; }\n\nfn _hidden() {}\n\n/// A point.\nstruct Point { x: i64; }\n\n@test\nfn t_x() {}\n' > docp/src/lib.mw
(cd docp && "$marrow" doc >/dev/null 2>&1) || t "doc status"
grep -q '^fn add(a: i64, b: i64) -> i64$' docp/build/doc/src.lib.md || t "doc signature: $(cat docp/build/doc/src.lib.md)"
grep -q '^Adds\.$' docp/build/doc/src.lib.md || t "doc text"
grep -q '^Demo\.$' docp/build/doc/src.lib.md || t "doc module text"
grep -q '_hidden\|t_x' docp/build/doc/src.lib.md && t "doc leaves out private names and tests"
grep -q 'src.lib.md' docp/build/doc/index.md || t "doc index"

# legacy form still builds
"$marrow" h.mw >/dev/null 2>&1 && [ -x ./h ] || t "legacy build"

# Windows: MSYS rewrites PATH and paths for native programs, these checks are about POSIX shells
case "$(uname -s)" in MINGW*|MSYS*) [ $fail = 0 ] && echo "cli ok"; exit $fail ;; esac

# tool discovery: qbe next to the binary, nothing else in PATH
mkdir bin; cp "$marrow" bin/marrow; cp "$(command -v qbe)" bin/qbe
PATH=/usr/bin:/bin ./bin/marrow doctor | grep -q "qbe: *$tmp/bin/qbe" || t "qbe next to marrow"
cp -r std bin/std
(cd / && PATH=/usr/bin:/bin "$tmp/bin/marrow" check "$tmp/h.mw" >/dev/null 2>&1) || t "std next to marrow"

# missing tools give actionable errors
MARROW_QBE=/nonexistent PATH=/nonexistent "$marrow" h.mw 2>&1 | grep -q '`qbe` was not found' || t "qbe missing message"
MARROW_CC=/nonexistent PATH=/nonexistent MARROW_QBE=$qbe_bin "$marrow" h.mw 2>&1 | grep -q 'no C compiler' || t "cc missing message"
# the standard library and the compiler follow their own conventions: no lint is reported
"$marrow" check "$root/std/std.mw" 2>&1 | grep -q 'warning' && t "std has no lint warnings"
"$marrow" check "$root/compiler/marrow.mw" 2>&1 | grep -q 'warning\[' && t "the compiler has no lint warnings"
[ $fail = 0 ] && echo "cli ok"
exit $fail
