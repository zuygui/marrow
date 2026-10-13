---
title: CLI reference
description: Everything the marrow command-line tool does, step by step.
sidebar:
  order: 1
---

The `marrow` binary is both the compiler front-end and the driver that invokes `qbe` and your system C toolchain. There are no subcommands: point it at a `.mw` file, get a binary (or object file, or assembly) out.

## Usage

```
marrow [OPTIONS] <input.mw> [output.ssa]
```

| Argument | Required | Description |
|---|---|---|
| `<input.mw>` | Yes | Path to the entry source file to compile. |
| `[output.ssa]` | No | Where to write the generated QBE IL. Defaults to `<input>` with its extension replaced by `.ssa` (e.g. `hello.mw` → `hello.ssa`). |

## Options

| Option | Description |
|---|---|
| `--check` | Type-check the program and stop. No code is generated and neither `qbe` nor a C compiler is needed. |
| `--nostdlib` | Link without libc: the program uses Marrow's own `_start` and system-call stub (see [Cross-compilation](/marrow/guides/cross-compilation/)). The whole [standard library](/marrow/standard-library/overview/) works in this mode. |
| `--target <arch>-<os>` | Compile for another target. `arch` is `x86_64`, `aarch64` or `riscv64`; `os` is `linux`, `macos` or `windows`. Defaults to the host. |
| `--color <when>` | Colorize diagnostics: `auto` (default), `always` or `never`. |
| `-V`, `--version` | Print `Marrow version: v<version>` and exit. |
| `-h`, `--help` | Print the usage text and exit. |

Running `marrow` with no input prints a usage message to stderr and exits with code `2`.

## What happens, in order

1. **Existence check.** If `<input.mw>` doesn't exist, prints an error and exits with code `2`.
2. **Parse and resolve imports.** Lexes and parses the entry file, follows every `@import(...)` (expanding `{target}` to the target's platform name, e.g. `linux_x86_64`) and flattens everything into one program. See [Modules & decorators](/marrow/language/decorators-and-modules/).
3. **Type checking.** Reports errors with Rust-style diagnostics (source excerpt, labels, notes, suggested fixes). Any error stops the build with exit code `1`. With `--check` the compiler stops successfully here.
4. **Code generation.** Walks the program and writes QBE IL to the `.ssa` file.
5. **QBE.** Runs `qbe -t <target> <output>.ssa -o <output>.s`. `qbe` must be on your `PATH`.
6. **Runtime assembly.** The generated assembly is extended with Marrow's runtime: the `__marrow_syscall` stub, the context-switch routine used by [tasks](/marrow/guides/async/), and, with `--nostdlib` on Linux, the `_start` entry point.
7. **Cross targets stop here.** When `--target` names something other than the host, `marrow` leaves the `.s` file and prints where it is; assembling and linking is up to you (see [Cross-compilation](/marrow/guides/cross-compilation/)).
8. **Assemble and link** with `cc` (`gcc` on Windows):
   - **Library** (`@no_main` somewhere in the program): `cc -c` to a `.o` file, then prints how to link it.
   - **Program:** links an executable (`-nostdlib -static` for freestanding Linux builds, `-lws2_32` on Windows).

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success (or `--version` / `--help`). |
| `1` | A compile-time error (parse, import or type error, code generation), a `qbe` failure, or a linker failure. |
| `2` | A usage error: missing argument, unknown option or target, input file not found, or failure to write the output. |

## Requirements

- `--check` needs nothing but `marrow` itself.
- A full build needs the `qbe` binary and a C compiler (`cc` on Linux/macOS, `gcc` on Windows) on `PATH` for the final assemble + link step. Without a supported compiler you still get the `.ssa` and `.s` files.
- Cross builds need `qbe` plus whatever assembler and linker targets that platform (for example `clang` and `ld.lld`).

## Examples

```bash
# Check a program without building anything
marrow --check app.mw

# A program with no libc at all (Linux)
marrow --nostdlib app.mw

# Assembly for 64-bit ARM Linux
marrow --nostdlib --target aarch64-linux app.mw
clang --target=aarch64-linux-gnu -nostdlib -static -fuse-ld=lld app.s -o app

# A library: std/std.mw is decorated @no_main, so this produces an object file
marrow std/std.mw
# -> std/std.o
```

This is how the project's CI works: every program in `tests/run/` is built with `--nostdlib`, run, and compared with its expected output, on Linux, macOS and Windows runners and (for aarch64 and riscv64) under qemu-user.
