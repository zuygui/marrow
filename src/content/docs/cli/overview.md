---
title: CLI reference
description: Everything the marrow command-line tool does, step by step.
sidebar:
  order: 1
---

The `marrow` binary is both the compiler front-end and the driver that invokes `qbe` and your system C compiler. There are no subcommands — it's a single-purpose tool: point it at a `.mw` file, get a binary (or object file) out.

## Usage

```
marrow <input.mw> [output.ssa]
marrow --version
```

| Argument | Required | Description |
|---|---|---|
| `<input.mw>` | Yes | Path to the entry source file to compile. |
| `[output.ssa]` | No | Where to write the generated QBE IL. Defaults to `<input>` with its extension replaced by `.ssa` (e.g. `hello.mw` → `hello.ssa`). |
| `--version` | — | Prints `Marrow version: v<cargo-package-version>` and exits immediately (ignores everything else). |

Running `marrow` with no arguments at all prints a usage message to stderr and exits with code `2`.

## What actually happens, in order

1. **Existence check.** If `<input.mw>` doesn't exist, prints an error and exits with code `2`.
2. **Parse + resolve imports** (`import::load_with_imports`) — lexes and parses the entry file, recursively follows every `@import(...)`, and flattens everything into a single `Program`. See [Modules & decorators](/marrow/language/decorators-and-modules/) for the exact resolution rules. Also determines whether the program is a **library** (any top-level item anywhere in the resolved program carries `@no_main`) or a standalone **binary**.
3. **Code generation** (`codegen::generate`) — walks the flattened program and emits QBE textual IL.
4. **Write the `.ssa` file** — the generated IL is written to the output path from step 0 (default or explicitly given).
5. **Invoke `qbe`** — runs `qbe <output>.ssa -o <output>.s` to turn the IL into native assembly. `qbe` must be reachable on your `PATH`.
6. **Pick a system C compiler** based on the OS `marrow` itself was built for: `gcc` on Windows, `cc` on macOS and Linux. On any other OS, the tool stops here — it prints *"Unsupported OS for compilation. Only QBE IL will be generated."* and leaves you with just the `.ssa`/`.s` files.
7. **Assemble & link:**
   - **If the program is a library** (`@no_main` present anywhere): runs `cc -c <output>.s -o <input-without-extension>.o`, then prints instructions for linking it into a final program yourself:
     ```
     Library compiled (no 'main', see '@no_main'): hello.o
     Link it into a program with: cc your_program.o hello.o -o your_program
     ```
   - **Otherwise**: runs `cc <output>.s -o <input-without-extension>` (no `-c`), directly producing a final executable, and prints:
     ```
     Compilation successful! Executable created at: hello
     ```

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success (or `--version` was requested). |
| `1` | A compile-time error (parse error, import error, codegen error), a `qbe` failure, or a linker failure. |
| `2` | A CLI usage error: missing argument, input file not found, or failure to write the output file. |

## Requirements at compile time

- The `qbe` binary must be on `PATH`.
- A C compiler (`cc` on Linux/macOS, `gcc` on Windows) must be on `PATH` for the final assemble+link step — otherwise you'll only get the intermediate `.ssa`/`.s` files.

## Example: compiling a library and linking it manually

```bash
# std/std.mw is decorated @no_main, so this produces an object file:
marrow std/std.mw
# -> std/std.o
# Link it into a program with: cc your_program.o std/std.o -o your_program

marrow my_app.mw
# my_app.mw (with @export fn main) compiles straight to an executable "my_app"
```

This two-step flow (compile a library to `.o`, compile your program, link both together with `cc`) is exactly how the project's own CI validates the standard library — see `.github/workflows/test-build.yaml`, which runs `cargo run -- std/std.mw` as its build check.