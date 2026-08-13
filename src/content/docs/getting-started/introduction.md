---
title: Introduction
description: What Marrow is, why it exists, and the philosophy behind the language.
sidebar:
  order: 1
---

Marrow is a fast, low-level, self-hosted systems programming language. It compiles down to [QBE](https://c9x.me/compile/) intermediate representation, which is then assembled and linked into a native executable (or object file) using your system's C toolchain.

:::caution[Early stage]
Marrow is under active development and **is not ready for production use**. The compiler itself is currently written in Rust while the language works toward self-hosting (a Marrow compiler written in Marrow). Large parts of the language surface described in this documentation already work end-to-end (lexing → parsing → QBE codegen → native binary), but the project is young, the standard library is small, and the syntax can still change.
:::

## Why "Marrow"?

Bone marrow is the living biological matrix nested deep within bones — it's responsible for generating the core vital cells necessary for an entire organism to function.

Marrow (the language) is built on that same idea: an ultra-lightweight, close-to-the-metal ("close-to-the-bone") system matrix. It aims to provide a solid, expressive foundation for building software without the overhead of a modern heavy runtime.

## Design philosophy

- **No hidden runtime.** There is no garbage collector, no implicit allocations, and no hidden control flow. If a Marrow program allocates memory, it's because *you* called an allocation function from the standard library.
- **Explicit memory management.** The standard library ships an arena allocator (see [`std/mem.mw`](/marrow/standard-library/mem/)) as the idiomatic way to manage memory in bulk, on top of raw `malloc`/`free` bindings.
- **A small, predictable core language.** Functions, structs, pointers, static arrays, slices, and the usual C-like control flow (`if`, `while`, `for`) — nothing more exotic than that today. No generics, no traits/interfaces, no closures, no operator overloading.
- **QBE as a backend, not an implementation detail you need to know.** Marrow generates textual QBE IL (`.ssa` files) and shells out to the `qbe` binary and to your system's C compiler (`cc`/`gcc`) to turn that into a real executable. This keeps the compiler itself small while still producing reasonably optimized native code.
- **C-friendly by default.** Calling into C libraries is a first-class use case: the `@extern` decorator lets you declare a foreign function and call it directly, which is exactly how the entire standard library is implemented (`malloc`, `printf`, `fopen`, etc. are all thin `@extern` wrappers).

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
│   Codegen    │  --> QBE IR generation (.ssa)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ QBE + cc/gcc │  --> Assembly (.s) ──> Executable / Object file
└──────────────┘
```

Each stage is implemented as its own Rust module in the compiler: `lexer.rs`, `parser.rs`, `ast.rs`, `import.rs`, `codegen.rs`, and `error.rs` for diagnostics. There is currently no separate type-checking/symbol-table pass — type resolution, scoping, and code generation all happen together inside `codegen.rs`.

## Where to go next

- [Installation](/marrow/getting-started/installation/) — install the `marrow` compiler and the `qbe` backend.
- [Hello World](/marrow/getting-started/hello-world/) — write, compile and run your first program.
- [Language reference](/marrow/language/overview/) — the full language guide.
- [CLI reference](/marrow/cli/overview/) — everything the `marrow` command can do.
- [Standard Library](/marrow/standard-library/overview/) — what ships in `std/`.