#!/bin/sh
# Pathological inputs: the compiler may reject them, but must neither crash nor hang.
set -u
root=$(cd "$(dirname "$0")/.." && pwd)
marrow=${MARROW:-$root/build/marrow}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
limit=""; command -v timeout >/dev/null 2>&1 && limit="timeout 60"
cd "$tmp"

# rep STRING N: STRING repeated N times
rep() { awk -v s="$1" -v n="$2" 'BEGIN { for (i = 0; i < n; i++) printf "%s", s }'; }

# case NAME: the program is on stdin
case_() {
    cat > "$tmp/$1.mw"
    $limit "$marrow" --check --color never "$tmp/$1.mw" >/dev/null 2>&1; st=$?
    if [ $st -ne 0 ] && [ $st -ne 1 ]; then echo "FAIL $1: exit status $st"; : > "$tmp/failed"; fi
}

printf '' | case_ empty
printf '// only a comment' | case_ comment
printf '/* never closed' | case_ unclosed_comment
printf '@' | case_ lone_at
printf 'fn main() { var s: *u8 = "never closed' | case_ unclosed_string
printf 'fn main() { var s: *u8 = "\\' | case_ dangling_escape
printf 'fn \000main() {}' | case_ nul_byte
printf 'fn main() { var s: *u8 = "\377\376\375"; }' | case_ invalid_utf8
printf 'fn main() { var x: i64 = 99999999999999999999999999999999; }' | case_ huge_int
printf 'fn main() { var x: f64 = 1e999999; var y: f64 = 0x1p99999; }' | case_ huge_float
{ printf 'fn main() { var x: i64 = '; rep '(' 20000; printf '1'; rep ')' 20000; printf '; }'; } | case_ deep_parens
{ printf 'fn main() '; rep '{ ' 20000; rep '} ' 20000; } | case_ deep_blocks
{ printf 'fn main() { var x: i64 = '; rep '-' 20000; printf '1; }'; } | case_ deep_unary
{ printf 'fn main() { var x: i64 = 1'; rep ' + 1' 50000; printf '; }'; } | case_ long_expression
{ printf 'fn main() { '; rep 'if true { ' 5000; rep '} ' 5000; printf '}'; } | case_ deep_if
{ printf 'fn f(a: '; rep '*' 20000; printf 'i64) {}'; } | case_ deep_pointer_type
{ printf 'fn f(a: fn('; rep 'fn(' 3000; rep ')' 3000; printf ') {}'; } | case_ deep_fn_type
{ printf 'fn main() { var '; rep 'a' 200000; printf ': i64 = 0; }'; } | case_ long_identifier
{ printf 'fn main() { var s: *u8 = "'; rep 'x' 1000000; printf '"; }'; } | case_ long_string
{ printf 'fn main() {\n'; rep 'var x: i64 = 1;\n' 30000; printf '}'; } | case_ many_statements
{ rep 'fn f() {}\n' 5000; } | case_ many_duplicate_functions
{ rep 'struct S { a: i64; }\n' 2000; } | case_ many_duplicate_structs
{ printf 'struct A { b: B; }\nstruct B { a: A; }\nfn main() {}'; } | case_ recursive_struct
{ printf 'struct A { a: A; }\nfn main() { var x: A; }'; } | case_ self_containing_struct
{ printf 'fn f() -> i64 { ret f(); }\nfn main() { f(); }'; } | case_ infinite_recursion
{ printf 'enum E { A; B(E); }\nfn main() { var e: E = E::B(E::A); }'; } | case_ recursive_enum
{ printf 'fn f<T>(x: T) { f<*T>(cast(*T) null); }\nfn main() { f::<i64>(1); }'; } | case_ infinite_generics
{ printf '@import("'; rep 'a/' 5000; printf 'b.mw")\nfn main() {}'; } | case_ long_import_path
{ printf '@import("self_import.mw")\nfn main() {}'; } | case_ self_import
{ printf 'fn main() { match x { '; rep 'E::A => {} ' 20000; printf '} }'; } | case_ many_match_arms
{ printf 'fn main() { var a: [i64; 99999999999999] ; }'; } | case_ huge_array
{ printf 'fn main() { var s: i64 = sizeof(['; rep '[' 10000; printf 'i64; 1]; 1]); }'; } | case_ deep_array_type

# a program that is valid but large still compiles
{ printf '@import("std/std.mw")\n'; i=0; while [ $i -lt 400 ]; do printf 'fn f%s(x: i64) -> i64 { ret x + %s; }\n' $i $i; i=$((i+1)); done
  printf '@export fn main(argc: i32, argv: rawptr) -> i32 { ret cast(i32) (f399(1) - 400); }\n'; } > big.mw
cp -r "$root/std" std
$limit "$marrow" --check --color never big.mw >/dev/null 2>"$tmp/big.err" || { echo "FAIL big valid program"; head -5 "$tmp/big.err"; : > "$tmp/failed"; }

[ ! -e "$tmp/failed" ] && echo "robust ok"
[ ! -e "$tmp/failed" ]
