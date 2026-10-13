---
title: Targets & cross-compilation
description: Which platforms Marrow supports, what --target and --nostdlib do, and how to build for another machine.
sidebar:
  order: 2
---

Marrow generates code through [QBE](https://c9x.me/compile/), which supports three architectures: **amd64** (`x86_64`), **arm64** (`aarch64`) and **rv64** (`riscv64`). Marrow adds operating-system layers on top, so a target is an `<arch>-<os>` pair:

| Target | `--nostdlib` | Status |
|---|---|---|
| `x86_64-linux` | yes | The reference platform; every test runs here. |
| `aarch64-linux` | yes | Assembled and linked; run under qemu-user in CI. |
| `riscv64-linux` | yes | Assembled and linked; run under qemu-user in CI. |
| `aarch64-macos` | yes | Assembled in CI, with a native macOS job configured. Experimental: never run by the author. |
| `x86_64-macos` | yes | As above. |
| `x86_64-windows` | yes | Assembled and linked into a PE file against kernel32, advapi32 and ws2_32. Experimental: never run by the author. |

32-bit ARM is not supported because QBE has no 32-bit backend.

## `--nostdlib`: programs without libc

With `--nostdlib` the program talks to the operating system itself. The compiler appends a small runtime to the generated assembly:

- `__marrow_syscall(n, a, b, c, d, e, f)`, the one primitive everything is built on. On Linux it is the raw system call; on macOS it is the BSD call (errors are turned into `-errno`); on Windows `n` indexes a table of Win32 functions.
- `_start`, on Linux, so the executable needs neither crt0 nor libc.
- `__marrow_ctx_switch`, the context switch used by [tasks](/marrow/guides/async/).

The standard library is written against that primitive and nothing else (see [the platform layer](/marrow/standard-library/platform/)), so the whole of it, including networking, TLS and HTTPS, works without libc.

## `--target`: building for another machine

```bash
marrow --nostdlib --target aarch64-linux app.mw
```

For a target other than the host, `marrow` stops after producing `app.s` (QBE assembly plus the runtime). You assemble and link it with the toolchain of your choice. Examples with LLVM:

```bash
# Linux on 64-bit ARM, freestanding
clang --target=aarch64-linux-gnu -nostdlib -static -fuse-ld=lld app.s -o app
qemu-aarch64 ./app

# Linux on RISC-V
marrow --nostdlib --target riscv64-linux app.mw
clang --target=riscv64-linux-gnu -nostdlib -static -fuse-ld=lld app.s -o app

# macOS (build the object on any machine, link on a Mac)
marrow --target aarch64-macos app.mw
clang --target=aarch64-apple-macos -c app.s -o app.o

# Windows
marrow --target x86_64-windows app.mw
clang --target=x86_64-pc-windows-gnu -c app.s -o app.o
# link with kernel32, advapi32 and ws2_32 (-lws2_32 with MinGW)
```

## How the standard library picks a platform

The `os.mw` module contains `@import("platform/{target}")`. The compiler replaces `{target}` with the target's name (`linux_x86_64`, `macos_aarch64`, `windows_x86_64`...), so a different file of `std/platform/` is compiled in for each target. See [the platform layer](/marrow/standard-library/platform/) for the contract those files implement.

## What is and isn't verified

Linux x86_64 is exercised by the full test suite on every change. The CI configuration also runs the end-to-end programs natively on Linux aarch64, macOS and Windows runners, and under qemu-user for aarch64 and riscv64, and assembles every target. The macOS and Windows layers were written from the platform documentation and have only been assembled and linked, not executed, so treat them as experimental until those CI jobs have run green.
