---
title: Decorators & modules
description: "@export, @extern, @import, @no_main, @test, @private, @allow, namespaces, and how the module/import system resolves files."
sidebar:
  order: 6
---

Decorators are written as `@name` or `@name(args...)` directly above a `fn`, `struct` or `var` declaration. A single declaration can carry several decorators (one per line, or stacked).

Seven decorators exist today: `@export`, `@extern`, `@import`, `@no_main`, `@test`, `@private` and `@allow`. Any other name is reported as warning `W0001` and ignored.

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

Declares a function that is implemented **outside** this Marrow program — in the C standard library, or in another object file or assembly routine you link in yourself. The standard library itself uses it for exactly two symbols supplied by the compiler's runtime: `__marrow_syscall` (the system-call primitive) and `__marrow_ctx_switch` (the context switch used by tasks). Everything else is written in Marrow, so programs do not need libc.

- The declaration **must not** have a body.
- Takes an optional single string argument: the actual linker symbol to bind to. If omitted, the symbol name is the same as the Marrow function name. This lets you give a foreign function a different, more convenient name on the Marrow side (`@extern("puts") fn c_puts(s: *u8) -> i32;`).
- `@extern` only applies to functions — using it on a `struct` or `var` is a compile error.
- `@extern` is incompatible with the expression-body form (`fn f() => ...;`) — use the block form without a body instead.

## `@no_main`

```marrow
@no_main

@extern("puts") fn c_puts(s: *u8) -> i32;
```

Tells the `marrow` CLI that this compilation unit is a **library**, not a standalone program — it doesn't need (and may not have) an entry point. When present, the CLI stops after producing an object file (`.o`) instead of trying to link a final executable, and prints instructions for linking it into another program yourself. See [CLI reference](/marrow/cli/overview/).

`@no_main` is a whole-file marker: it can technically decorate *any* top-level declaration (it just needs to appear somewhere in the entry file's item list — the compiler doesn't care which declaration it's attached to), but the convention used throughout the standard library is to place it alone at the very top of the file, immediately decorating whatever the first declaration happens to be.

## `@import`

```marrow
@import("std")
@import("string.mw")
@import("../shared/utils")
```

Brings another file's top-level declarations into scope, as if they were textually included at that point. Takes exactly one string argument: the import target.

### `{target}` in import paths

The text `{target}` inside an import string is replaced by the name of the compilation target (`linux_x86_64`, `macos_aarch64`, `windows_x86_64`...). The standard library uses it to pick the right [platform layer](/marrow/standard-library/platform/): `@import("platform/{target}")`.

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

### Namespaces

By default an import is flat: everything lands in one global set of names. Give the import a second argument, an **alias**, to keep a module's names apart:

```marrow
@import("std/std.mw")
@import("lib/shapes.mw", shapes)
@import("json", json)

fn main_value() -> i64 {
    var p: shapes::Point = shapes::make(1, 2);
    ret shapes::area(p);
}
```

- Functions, structs, enums, constants and globals of the module are reached as `alias::name`, in expressions and in types. Enum variants are `alias::Shape::Circle(2)`, and generic calls are `alias::id::<i64>(42)`.
- Inside the module, its own names need no prefix. Two modules (or the program and a module) can declare the same name without clashing.
- What the module imports itself, with a plain `@import`, stays inside the module.
- A **package** (a folder with a `marrow.toml`) is imported by name: `@import("json", json)` loads the package's `lib` file, and every file of its directory shares one namespace. See [Packages](/marrow/guides/packages/).
- Using a name that is not in the module is error `E0006` ("module `alias` has no item named ..."), and importing a package that is not a dependency of the project gives a diagnostic that tells you to `marrow add` it.

### Deduplication & ordering

- Every file is loaded **at most once** per compilation, tracked by canonicalized absolute path — importing the same file from multiple places (directly or transitively) is safe and free.
- Because of that dedup rule, **circular imports don't error out** — if file A imports B and B imports A, whichever one is loaded first simply "wins" the first pass, and the second file's re-import of it is silently skipped. Don't rely on both sides of a cycle being able to see each other's declarations in every order; keep top-level names distinct instead.
- Imports are resolved depth-first, and each imported file's declarations are spliced into the flat program **before** the item that declared the `@import`. Multiple `@import`s in one file are processed in the order they're written.
- A plain `@import` has no namespace: everything imported lands in one global set of function/struct/const names, and a collision across files is a compile error the same way a local redefinition would be. Use an alias to avoid that.

## `@private`

```marrow
@private fn helper() -> i64 { ret 1; }
@private var CACHE: i64 = 0;
```

Marks a function, struct, enum, constant or global as reachable **inside its module only**. Another file that imports the module with an alias and writes `alias::helper` gets error `E0007` ("`helper` is private to module `alias`"), with a hint to remove `@private` if it was meant to be used from outside. Code in the module itself is not affected.

## `@test`

```marrow
@import("std/std.mw")
@import("std/test.mw")

@test fn adds() {
    check_eq_i64(1 + 1, 2, "one plus one");
}
```

Marks a function with no parameters as a test. Tests are not part of normal builds: `marrow test` compiles each file that holds tests, runs every `@test` function in its own process and reports the result. `std/test.mw` has the helpers: `check(cond, message)`, `check_eq_i64(actual, expected, message)`, `check_eq_str(actual, expected, message)` and `test_fail(message)`. A failed check prints what went wrong and ends the process with status `1`. See [`marrow test`](/marrow/guides/testing/).

## `@allow`

```marrow
@allow(unused)
fn scratch() {
    var a: i64 = 1;      // no warning in this function
}

fn other() {
    @allow(unused) var b: i64 = 2;   // no warning for this variable
    var _c: i64 = 3;                  // a leading underscore does the same
}
```

The compiler warns (`W0007`) about a local variable that is declared and never used. `@allow(unused)` silences that warning, either for a whole function (written above the `fn`) or for one local variable (written in front of its `var`). A name that starts with `_` is never reported either. Assigning to a variable counts as using it.

`unused` is the only argument that exists today; another one is reported as warning `W0001`. In front of a statement, `@allow(unused)` must be followed by a `var`, and no other decorator is accepted there (error `E0207`).

## What decorators *aren't*

There's no `@inline`, no `@packed`/alignment control, no visibility modifiers beyond `@private`, and no attribute macros. The decorator list above is exhaustive for the current compiler.