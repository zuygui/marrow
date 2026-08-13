---
title: Hello, World!
description: Write, compile and run your first Marrow program.
sidebar:
  order: 3
---

## Write the program

Create a file called `hello.mw`:

```marrow title="hello.mw"
@import("std")

@export fn main(argc: i32, argv: rawptr) -> i32 {
    println("Hello from Marrow !");

    print("Received arg count: ");
    println_i64(cast(i64) argc);

    ret 0;
};
```

A few things worth noticing already (all covered in depth in the [Language reference](/marrow/language/overview/)):

- `@import("std")` pulls in the standard library's umbrella module (`std/std.mw`), which in turn imports `io.mw`, `mem.mw`, `string.mw`, `sys.mw`, `vec.mw` and `map.mw` for you. This is what makes `println`, `print` and `println_i64` available.
- `@export` marks `main` as an externally-visible symbol — required for the C linker to find your entry point.
- `argc: i32, argv: rawptr` mirrors the C `int argc, char** argv` signature, since Marrow programs are linked against the C runtime's `_start`/`main` machinery.
- `cast(i64) argc` explicitly converts the `i32` to an `i64` before it's passed to `println_i64`.
- Every statement ends in `;`, including the closing `}` of the top-level `fn` declaration.

## Compile it

```bash
marrow hello.mw
```

This single command runs the whole pipeline:

1. Parses `hello.mw` and resolves its `@import`s.
2. Generates QBE IL into `hello.ssa` (the default output path — see [CLI reference](/marrow/cli/overview/) to override it).
3. Invokes `qbe hello.ssa -o hello.s` to produce native assembly.
4. Since the program is **not** a library (no `@no_main` anywhere in the file), invokes your system C compiler to assemble and link `hello.s` directly into an executable named after the input file with its extension stripped: `hello`.

On success you'll see:

```
Compilation successful! Executable created at: hello
```

## Run it

```bash
./hello arg1 arg2
```

```
Hello from Marrow !
Received arg count: 3
```

(`argc` is `3` because the program's own name counts as `argv[0]`, per the usual C convention.)

## Compiling a library instead of a binary

If your file is decorated with `@no_main` anywhere (this is how every file in `std/` is written), `marrow` treats it as a **library**: it stops after producing an object file instead of linking an executable, and tells you how to link it yourself:

```
Library compiled (no 'main', see '@no_main'): hello.o
Link it into a program with: cc your_program.o hello.o -o your_program
```

See [Modules & decorators](/marrow/language/decorators-and-modules/) for the full explanation of `@no_main`, `@export`, `@extern` and `@import`.