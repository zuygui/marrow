---
title: Introduction
description: What Marrow is, why it exists, and the philosophy behind the language.
sidebar:
  order: 1
---

Marrow is a fast, low-level, self-hosted systems programming language. It compiles down to [QBE](https://c9x.me/compile/) intermediate representation, which is then assembled and linked into a native executable (or object file) using your system's C toolchain.

:::caution[Early stage]
Marrow is under active development and **is not ready for production use**. The reference compiler is written in Rust, and a complete second compiler written in Marrow (`compiler/marrow.mw`) now bootstraps: the Rust-built compiler builds it, it builds itself, and the QBE IL and assembly of every generation are identical. Its diagnostics (snippets, labels, notes, suggestions, colors) are rendered byte for byte like the Rust ones on every error case of the repository, and only Linux x86_64 is exercised end to end. The language and a large, libc-free standard library (networking, TLS and HTTPS, cryptography, JSON, async tasks...) already work end-to-end, but the project is young and the syntax can still change.
:::

## Why "Marrow"?

Bone marrow is the living biological matrix nested deep within bones — it's responsible for generating the core vital cells necessary for an entire organism to function.

Marrow (the language) is built on that same idea: an ultra-lightweight, close-to-the-metal ("close-to-the-bone") system matrix. It aims to provide a solid, expressive foundation for building software without the overhead of a modern heavy runtime.

## Design philosophy

- **No hidden runtime.** There is no garbage collector, no implicit allocations, and no hidden control flow. If a Marrow program allocates memory, it's because *you* called an allocation function from the standard library.
- **Explicit memory management.** The standard library ships an arena allocator (see [`std/mem.mw`](/marrow/standard-library/mem/)) as the idiomatic way to manage memory in bulk, on top of a heap allocator written in Marrow.
- **A small, predictable core language.** Functions and function pointers, structs, enums with payloads and exhaustive `match`, generics (explicit type arguments), pointers, static arrays, slices, `sizeof`, and the usual C-like control flow (`if`, `while`, `for`, `break`, `continue`). No traits/interfaces, no closures, no operator overloading.
- **QBE as a backend, not an implementation detail you need to know.** Marrow generates textual QBE IL (`.ssa` files) and shells out to the `qbe` binary and to your system's C compiler (`cc`/`gcc`) to turn that into a real executable. This keeps the compiler itself small while still producing reasonably optimized native code, for x86_64, aarch64 and riscv64.
- **No libc required.** The standard library talks to the operating system directly through a small [platform layer](/marrow/standard-library/platform/) (Linux, macOS and Windows), so a Marrow program can be a fully static executable. Calling C is still easy: the `@extern` decorator declares a foreign function you can call directly.

## What the language looks like

```marrow
@import("std/std.mw")

@export fn main (argc: i32, argv: rawptr) -> i32 {
    println("Hello from Marrow !");

    print("Received arg count: ");
    println_i64(cast(i64) argc);

    ret 0;
};
```

## Compilation pipeline

Marrow is organized into clearly separated stages:

```
 Source Code (.mw)
        │
        ▼
┌──────────────┐
│    Lexer     │  --> Tokenization (identifiers, literals, operators)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│    Parser    │  --> Abstract Syntax Tree (AST)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Imports    │  --> Resolves '@import(...)' and flattens the module graph
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ Type checker │  --> Diagnostics (Rust-style errors and warnings)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Codegen    │  --> QBE IR generation (.ssa)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ QBE + cc/gcc │  --> Assembly (.s) ──> Executable / Object file
└──────────────┘
```

Each stage is implemented as its own Rust module in the compiler: `lexer.rs`, `parser.rs`, `ast.rs`, `import.rs`, `typeck.rs`, `codegen.rs`, `runtime.rs` (the per-target runtime assembly) and `diagnostic.rs`. The type checker runs before code generation, so `marrow --check` can validate a program without QBE or a C compiler.

## Where to go next

- [Installation](/marrow/getting-started/installation/) — install the `marrow` compiler and the `qbe` backend.
- [Your first Marrow program](/marrow/getting-started/your-first-marrow-program/) — write, compile and run a program.
- [Targets & cross-compilation](/marrow/guides/cross-compilation/) — Linux, macOS, Windows on x86_64, aarch64 and riscv64.
- [Async tasks](/marrow/guides/async/) — coroutines, channels and non-blocking I/O.
- [Language reference](/marrow/language/overview/) — the full language guide.
- [CLI reference](/marrow/cli/overview/) — everything the `marrow` command can do.
- [Standard Library](/marrow/standard-library/overview/) — what ships in `std/`.