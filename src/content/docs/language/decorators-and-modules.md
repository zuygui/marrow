---
title: Decorators & modules
description: "@export, @extern, @import, @no_main, and how the module/import system resolves files."
sidebar:
  order: 6
---

Decorators are written as `@name` or `@name(args...)` directly above a `fn`, `struct` or `var` declaration. A single declaration can carry several decorators (one per line, or stacked).

Only four decorators exist today: `@export`, `@extern`, `@import` and `@no_main`.

## `@export`

```marrow
@export fn main(argc: i32, argv: rawptr) -> i32 { ... }
@export var PI: f64 = 3.14159;
```

Marks a function or global variable's symbol as **exported** (visible for external linking — equivalent to a non-`static` symbol in C). Without `@export`, top-level functions/globals still compile fine and can be called from within the same compiled program, but their symbol isn't specially marked for external linkage. In practice, your program's `main` (or any function you intend to call from a separately-compiled `.o` file) needs `@export`.

`@export` can be combined with `@extern`, but note that `@extern` functions never generate a body — `@export` on an `@extern` declaration is accepted by the parser but has no real effect since there's no definition to export.

## `@extern`

```marrow
@extern fn my_c_function(x: i32) -> i32;
@extern("actual_symbol_name") fn friendly_name(x: i32) -> i32;
```

Declares a function that is implemented **outside** this Marrow program — typically in the C standard library, or in another object file you'll link in yourself. The whole standard library is built this way (`c_malloc`, `c_printf`, `c_fopen`, etc. all wrap libc functions).

- The declaration **must not** have a body.
- Takes an optional single string argument: the actual linker symbol to bind to. If omitted, the symbol name is the same as the Marrow function name. This lets you give a foreign function a different, more convenient name on the Marrow side (`@extern("puts") fn c_puts(s: *u8) -> i32;`).
- `@extern` only applies to functions — using it on a `struct` or `var` is a compile error.
- `@extern` is incompatible with the expression-body form (`fn f() => ...;`) — use the block form without a body instead.

## `@no_main`

```marrow
@no_main

@extern("puts") fn c_puts(s: *u8) -> i32;
```

Tells the `marrow` CLI that this compilation unit is a **library**, not a standalone program — it doesn't need (and may not have) an entry point. When present, the CLI stops after producing an object file (`.o`) instead of trying to link a final executable, and prints instructions for linking it into another program yourself. See [CLI reference](/cli/overview/).

`@no_main` is a whole-file marker: it can technically decorate *any* top-level declaration (it just needs to appear somewhere in the entry file's item list — the compiler doesn't care which declaration it's attached to), but the convention used throughout the standard library is to place it alone at the very top of the file, immediately decorating whatever the first declaration happens to be.

## `@import`

```marrow
@import("std")
@import("string.mw")
@import("../shared/utils")
```

Brings another file's top-level declarations into scope, as if they were textually included at that point. Takes exactly one string argument: the import target.

### Resolution order

For a target like `"foo"`, the resolver tries, **in this order**, until one succeeds:

1. **The importing file's own directory.**
2. **The entry file's directory** (the file you originally passed to `marrow` on the command line), if different from #1.
3. **`~/.marrow/std`**, if that directory exists (this is where `install.sh` copies the bundled standard library).
4. **`~/.marrow`** itself, if it exists.

*(On Windows, `~` resolves via the `USERPROFILE` environment variable instead of `HOME`.)*

Within each of those base directories, it tries, in order:

1. The target path exactly as written, joined to the base (`base/foo`).
2. If the target doesn't already end in `.mw` or `.mrw`: `base/foo.mw`, then `base/foo.mrw`.
3. If the target (without any extension appended) resolves to a **directory**: an "umbrella" file named after that directory, i.e. `base/foo/foo.mw`, then `base/foo/foo.mrw`.

This is exactly why `@import("std")` works both when the standard library is installed flat into `~/.marrow/std` (rule 2, matching `std.mw` inside that directory) *and* when you keep the repository's `std/` folder next to your own source file (rule 3, matching the `std/std.mw` umbrella file).

If none of the candidate paths exist, compilation fails with a list of every path that was tried.

### Deduplication & ordering

- Every file is loaded **at most once** per compilation, tracked by canonicalized absolute path — importing the same file from multiple places (directly or transitively) is safe and free.
- Because of that dedup rule, **circular imports don't error out** — if file A imports B and B imports A, whichever one is loaded first simply "wins" the first pass, and the second file's re-import of it is silently skipped. Don't rely on both sides of a cycle being able to see each other's declarations in every order; keep top-level names distinct instead.
- Imports are resolved depth-first, and each imported file's declarations are spliced into the flat program **before** the item that declared the `@import`. Multiple `@import`s in one file are processed in the order they're written.
- There is currently no namespacing — everything imported lands in one single global set of function/struct/const names. Name collisions across files are a compile error the same way a local redefinition would be.

## What decorators *aren't*

There's no `@inline`, no `@packed`/alignment control, no visibility modifiers beyond `@export`, and no attribute macros. The decorator list above (`@export`, `@extern`, `@import`, `@no_main`) is exhaustive for the current compiler.