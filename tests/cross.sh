#!/bin/sh
# Builds every tests/run/*.mw for another architecture (freestanding Linux),
# runs it under qemu-user and compares its output with the .stdout file.
#
#   tests/cross.sh aarch64-linux aarch64-linux-gnu qemu-aarch64
#   tests/cross.sh riscv64-linux riscv64-linux-gnu qemu-riscv64
#
# Needs qbe, clang, ld.lld and the given qemu-user binary. Set NO_RUN=1 to
# only build and link (useful where no emulator is installed).
set -eu

target=$1
triple=$2
qemu=${3:-}
root=$(cd "$(dirname "$0")/.." && pwd)
marrow=${MARROW:-$root/build/marrow}
work=$root/build/cross-$target
rm -rf "$work"
mkdir -p "$work"
cp -r "$root/std" "$work/std"

fail=0
for src in "$root"/tests/run/*.mw; do
    name=$(basename "$src" .mw)
    cp "$src" "$work/$name.mw"
    if ! (cd "$work" && "$marrow" --nostdlib --color never --target "$target" "$name.mw" >/dev/null 2>"$name.log"); then
        echo "FAIL $name: marrow failed"; cat "$work/$name.log"; fail=1; continue
    fi
    if ! clang --target="$triple" -nostdlib -static -fuse-ld=lld "$work/$name.s" -o "$work/$name" 2>"$work/$name.log"; then
        echo "FAIL $name: link failed"; cat "$work/$name.log"; fail=1; continue
    fi
    if [ -n "${NO_RUN:-}" ]; then
        echo "built $name"
        continue
    fi
    (cd "$work" && "$qemu" "./$name" >"$name.out" 2>"$name.log") || { echo "FAIL $name: exit status $?"; cat "$work/$name.log"; fail=1; continue; }
    if cmp -s "$work/$name.out" "$root/tests/run/$name.stdout"; then
        echo "ok   $name"
    else
        echo "FAIL $name: output differs"; diff "$root/tests/run/$name.stdout" "$work/$name.out" | head -20; fail=1
    fi
done
exit $fail
