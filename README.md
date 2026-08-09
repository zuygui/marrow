![Marrow is a fast, self-hosted, systems programming language](https://raw.githubusercontent.com/zuygui/marrow/refs/heads/main/.github/assets/banner-light.svg#gh-light-mode-only)
![Marrow is a fast, self-hosted, systems programming language](https://raw.githubusercontent.com/zuygui/marrow/refs/heads/main/.github/assets/banner-dark.svg#gh-dark-mode-only)

<p align="center">
  <b>A fast, low-level, self-hosted systems programming language.</b>
</p>

> [!IMPORTANT]
> The project is still under active development and is not ready for production use.

## 🧬 Why "Marrow"?

**Bone marrow** is the living biological matrix nested deep within bones. It is responsible for generating the core vital cells necessary for the entire organism to function and thrive.

**Marrow** was built on that exact philosophy: to serve as an ultra-lightweight, close-to-the-metal ("close-to-the-bone") system matrix. It provides a solid and expressive foundation for building softwares without the overhead of modern heavy runtimes.

## ⚡ Key Features

- **Self-Hosted:** The Marrow compiler is written in Marrow (currently in Rust but WIP).
- **QBE Backend:** Leverages [QBE](https://c9x.me/compile/) as an intermediate representation (IR) code generator for blazingly fast compilation and a minimal footprint.
- **Arena Memory Allocation:** Explicit and ultra-performant memory management by default.
- **Zero Runtime / Freestanding:** No garbage collector, no hidden runtime cost.
- **Clean Syntax:** Strongly and explicitly typed language inspired by C and Rust.

## 💻 Code example

```marrow
@import("std/std.mrw")

@export main :: (i32 argc, rawptr argv) -> i32 {
    printn("Hello from Marrow !");

    print("Received arg count: ");
    println_i64(cast(i64) argc);

    ret 0;
};
```

## 🏗️ Architecture & How It Works Under the Hood

Marrow is designed modularly to ensure fast compilation times and a clean codebase that is easy to evolve:

```text
  Source Code (.mrw)
         │
         ▼
 ┌──────────────┐
 │    Lexer     │  --> Tokenization & Keyword Lookup (ArrayMap)
 └──────┬───────┘
        │
        ▼
 ┌──────────────┐
 │    Parser    │  --> Abstract Syntax Tree (AST) & Arena Allocations
 └──────┬───────┘
        │
        ▼
 ┌──────────────┐
 │  SymTable    │  --> Symbol Resolution & Scopes (LinearMap)
 └──────┬───────┘
        │
        ▼
 ┌──────────────┐
 │   Codegen    │  --> QBE IR Generation (.qbe)
 └──────┬───────┘
        │
        ▼
 ┌──────────────┐
 │ QBE + Linker │  --> Assembly Code (.s) ──> Executable / Binary
 └──────────────┘
 ```

## 🗺️ Roadmap to Self-Hosting

- [x] CLI & Arguments: Full argc / argv support.
- [x] Standard Library: Arena Allocator, VecPtr, HashMap (DJB2), LinearMap, and ArrayMap.
- [ ] Lexer / Tokenizer: Lexical analysis and symbol scanner.
- [ ] Parser & AST: Abstract Syntax Tree construction.
- [ ] Symbol Table: Scope handling and type checking.
- [ ] QBE Codegen: IR generation targeting the QBE backend.
- [ ] Full Self-Hosting: Recompiling the compiler using itself (marrow_v1 compiles marrow_v2).

## Thanks to all contributors :heart:

<p align="center">
  <img src="https://contrib.rocks/image?repo=zuygui/marrow" />
</p>