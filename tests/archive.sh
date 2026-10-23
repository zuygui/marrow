#!/bin/sh
# gzip and tar support of the standard library: archives made by the system `tar` and `gzip`
# are extracted by std/inflate.mw and std/tar.mw and compared with the originals.
set -u
root=$(cd "$(dirname "$0")/.." && pwd)
marrow=${MARROW:-$root/build/marrow}
command -v tar >/dev/null 2>&1 && command -v gzip >/dev/null 2>&1 || { echo "archive: tar or gzip not found, skipped"; exit 0; }
tmp=$(cd "$(mktemp -d)" && pwd -P)
trap 'rm -rf "$tmp"' EXIT
fail=0
t() { echo "FAIL $1"; fail=1; }
exe=""; case "$(uname -s)" in MINGW*|MSYS*) exe=.exe ;; esac

cp -r "$root/std" "$tmp/std"; cp "$root/tests/archive/untar.mw" "$tmp/untar.mw"
(cd "$tmp" && "$marrow" untar.mw >/dev/null 2>"$tmp/err") || { t "build untar"; cat "$tmp/err"; exit 1; }

# a tree with text, binary, empty files, deep folders and a long name
mkdir -p "$tmp/src/pkg/a/b/c"; cd "$tmp/src"
cp "$root/compiler/pkg.mw" pkg/a/pkg.mw
head -c 20000 /dev/urandom > pkg/a/b/random.bin
: > pkg/empty
echo "hello" > pkg/a/b/c/hello.txt
long=$(printf 'x%.0s' $(seq 1 120)); echo long > "pkg/a/b/c/$long.txt"
for level in 1 6 9; do
    tar cf - pkg | gzip -$level > "$tmp/t$level.tgz"
    rm -rf "$tmp/out"; mkdir "$tmp/out"
    "$tmp/untar$exe" "$tmp/t$level.tgz" "$tmp/out" >/dev/null 2>&1 || t "extract level $level"
    diff -r "$tmp/out" "$tmp/src/pkg" >/dev/null 2>&1 || t "contents differ at level $level"
done
# stored (uncompressed) deflate blocks
tar cf - pkg | gzip -1 > /dev/null
head -c 3000 /dev/urandom > pkg/stored.bin
tar cf - pkg | gzip -1 > "$tmp/s.tgz"; rm -rf "$tmp/out"; mkdir "$tmp/out"
"$tmp/untar$exe" "$tmp/s.tgz" "$tmp/out" >/dev/null 2>&1 && diff -r "$tmp/out" "$tmp/src/pkg" >/dev/null 2>&1 || t "incompressible data"

# damaged data is refused, not crashed on
head -c 500 "$tmp/t6.tgz" > "$tmp/trunc.tgz"
"$tmp/untar$exe" "$tmp/trunc.tgz" "$tmp/out" >/dev/null 2>&1 && t "truncated archive accepted"
printf 'not gzip at all' > "$tmp/bad.tgz"
"$tmp/untar$exe" "$tmp/bad.tgz" "$tmp/out" >/dev/null 2>&1 && t "garbage accepted"

# paths that climb out of the destination are refused (GNU tar only)
if tar --version 2>/dev/null | grep -q 'GNU tar'; then
    mkdir -p "$tmp/e"; cd "$tmp/e"; echo x > f
    tar czf "$tmp/evil.tgz" --transform='s,^,top/../../,' f 2>/dev/null
    rm -rf "$tmp/out"; mkdir "$tmp/out"
    "$tmp/untar$exe" "$tmp/evil.tgz" "$tmp/out" >"$tmp/err" 2>&1 && t "path with .. accepted"
    grep -q '\.\.' "$tmp/err" || t "message for .. paths"
    [ -f "$tmp/f" ] && t "file written outside of the destination"
    tar czPf "$tmp/abs.tgz" "$tmp/e/f" 2>/dev/null
    "$tmp/untar$exe" "$tmp/abs.tgz" "$tmp/out" >/dev/null 2>&1 && t "absolute path accepted"
fi
[ $fail = 0 ] && echo "archive ok"
exit $fail
