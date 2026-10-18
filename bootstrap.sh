#!/bin/sh
# Builds the Marrow compiler (compiler/marrow.mw, written in Marrow) without
# any other Marrow compiler: a checked-in assembly seed is assembled into
# stage 0, which compiles the sources into stage 1, which compiles them again
# into stage 2. Stage 1 and stage 2 must be identical (fixed point).
#
#   ./bootstrap.sh                 build build/marrow
#   ./bootstrap.sh --update-seed   after a successful build, regenerate bootstrap/seed-*.s.gz
#
# Needs: cc (gcc on Windows/MSYS2), qbe, gzip.
set -eu
cd "$(dirname "$0")"
root=$(pwd)

case "$(uname -s)" in
    Linux) os=linux ;;
    Darwin) os=macos ;;
    MINGW*|MSYS*) os=windows ;;
    *) echo "unsupported OS: $(uname -s)" >&2; exit 1 ;;
esac
case "$(uname -m)" in
    x86_64|amd64) arch=x86_64 ;;
    aarch64|arm64) arch=aarch64 ;;
    riscv64) arch=riscv64 ;;
    *) echo "unsupported architecture: $(uname -m)" >&2; exit 1 ;;
esac
target=$arch-$os
cc=cc; exe=""; link_extra=""
if [ "$os" = windows ]; then cc=gcc; exe=.exe; link_extra="-lws2_32 -lcrypt32 -Xlinker --stack -Xlinker 67108864,67108864"; fi
if [ "$os" = linux ]; then link_extra="-nostdlib -static -no-pie"; fi
seed=bootstrap/seed-$target.s.gz
[ -f "$seed" ] || { echo "no bootstrap seed for $target ($seed)" >&2; exit 1; }

mkdir -p build
echo "stage 0: assembling $seed"
gzip -dc "$seed" > build/s0.s
# shellcheck disable=SC2086
$cc build/s0.s -o build/s0$exe $link_extra

# stage N: compile the compiler with stage N-1 in a scratch copy of the tree
# (the compiler writes its outputs next to the source file).
stage() {
    n=$1; prev=$2
    echo "stage $n: compiling compiler/marrow.mw"
    rm -rf "build/s$n"
    mkdir -p "build/s$n"
    cp -r std compiler "build/s$n/"
    (cd "build/s$n/compiler" && "$root/$prev" --nostdlib --color never marrow.mw "$root/build/s$n.ssa") >/dev/null
    cp "build/s$n/compiler/marrow$exe" "build/s$n.bin$exe"
}
stage 1 "build/s0$exe"
stage 2 "build/s1.bin$exe"

if ! cmp -s build/s1.s build/s2.s || ! cmp -s build/s1.ssa build/s2.ssa; then
    echo "error: stage 1 and stage 2 differ: the compiler does not reach a fixed point" >&2
    exit 1
fi
if ! gzip -dc "$seed" | cmp -s - build/s1.s; then
    msg="the checked-in seed ($seed) is older than the sources; run ./bootstrap.sh --update-seed"
    if [ -n "${STRICT_SEED:-}" ]; then echo "error: $msg" >&2; exit 1; fi
    echo "warning: $msg" >&2
fi
cp "build/s2.bin$exe" "build/marrow$exe"
echo "built build/marrow$exe (fixed point reached)"

if [ "${1:-}" = "--update-seed" ]; then
    for t in x86_64-linux aarch64-linux riscv64-linux aarch64-macos x86_64-macos x86_64-windows; do
        rm -rf build/seed
        mkdir -p build/seed
        cp -r std compiler build/seed/
        (cd build/seed/compiler && "$root/build/marrow$exe" --nostdlib --color never --target "$t" marrow.mw "$root/build/seed-$t.ssa") >/dev/null 2>&1 || true
        gzip -9 -n -c "build/seed-$t.s" > "bootstrap/seed-$t.s.gz"
        echo "updated bootstrap/seed-$t.s.gz"
    done
fi
