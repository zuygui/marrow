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

- **Self-Hosted:** The Marrow compiler is written in Marrow (`compiler/marrow.mw`). The original Rust compiler lives on the [`v1`](https://github.com/zuygui/marrow/tree/v1) branch.
- **QBE Backend:** Leverages [QBE](https://c9x.me/compile/) as an intermediate representation (IR) code generator for blazingly fast compilation and a minimal footprint.
- **Arena Memory Allocation:** Explicit and ultra-performant memory management by default.
- **Zero Runtime / Freestanding:** No garbage collector, no hidden runtime cost.
- **Checked memory, no runtime:** owning pointers (`^T`) are freed automatically and moves are checked, borrows (`&T`, `&mut T`) are checked within a block, and raw pointers (`*T`, `raw x`) stay available for everything else.
- **Tooling included:** `marrow fmt`, `marrow doc`, a language server (`marrow lsp`), lints, a package manager and a registry, all in the one binary.
- **Clean Syntax:** Strongly and explicitly typed language inspired by C and Rust.

## 💻 Code example

```marrow
@import("std/std.mrw")

@export fn main (argc: i32, argv: rawptr) -> i32 {
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
- [x] Standard Library: libc-free (files, directories, sockets, HTTP/HTTPS, TLS 1.3, hashing and ciphers, big integers, RSA/ECDSA, JSON, math, time, async tasks); see the [docs](https://zuygui.github.io/marrow/standard-library/overview/).
- [x] Function pointers.
- [x] Language features for self-hosting: `break` / `continue`, `sizeof`, zero-initialized locals (`var x: T;`), enums with payloads and exhaustive `match`, generics.
- [x] Platforms: Linux (x86_64, aarch64, riscv64); macOS and Windows layers are experimental.
- [x] Lexer / Tokenizer: ported to Marrow (`compiler/lexer.mw`), checked token by token against the Rust lexer while porting (see `v1`).
- [x] Parser & AST: ported to Marrow (`compiler/parser.mw`), checked against the Rust parser on every `.mw` file, errors included.
- [x] Module loader and generics/enum lowering: ported to Marrow (`compiler/import.mw`, `compiler/lower.mw`), compared with the Rust passes on every `.mw` file.
- [x] Symbol Table: scope handling and type checking, ported to Marrow (`compiler/typeck.mw`); diagnostics carry labels, notes, helps and suggestions like the Rust ones.
- [x] QBE Codegen: ported to Marrow (`compiler/codegen.mw`); the generated IL is byte-for-byte identical to the Rust compiler's on every program of the repo.
- [x] Full Self-Hosting: `compiler/marrow.mw` is the whole compiler in Marrow. The Rust-built compiler (marrow_v1) builds it (marrow_v2), which builds it again (marrow_v3): the QBE IL and assembly are identical (`./bootstrap.sh`, which starts from the checked-in assembly seeds in `bootstrap/` and needs only `cc` and `qbe`; the Rust compiler on the `v1` branch is no longer needed). The Marrow driver renders diagnostics (snippets, labels, notes, suggestions, colors) byte for byte like the Rust one on every error case of the repository (`--color never` and `always`); `--color auto` checks that stderr is a character device, `NO_COLOR` and `TERM` (it cannot tell `/dev/null` from a terminal). Only Linux x86_64 is exercised end to end.

## Thanks to all contributors :heart:

<p align="center">
  <img src="https://contrib.rocks/image?repo=zuygui/marrow" />
</p>

## Building from source

```sh
./bootstrap.sh           # seed -> stage 1 -> stage 2, writes build/marrow
tests/diag.sh            # golden diagnostics
tests/run.sh             # end-to-end programs
```

`bootstrap.sh --update-seed` regenerates `bootstrap/seed-*.s.gz` after the compiler changes.
