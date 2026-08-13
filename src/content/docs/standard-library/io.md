---
title: "io — printing"
description: Console output functions from std/io.mw.
sidebar:
  order: 2
---

Source: `std/io.mw`. No imports of its own — this is the base module most other output-producing code builds on.

## Foreign bindings

| Function | Binds to (C) |
|---|---|
| `c_puts(s: *u8) -> i32` | `puts` |
| `c_printf(fmt: *u8, val: rawptr) -> i32` | `printf` |
| `c_fputs(s: *u8, stream: rawptr) -> i32` | `fputs` |
| `c_stderr() -> rawptr` | `stderr` |

These are internal — use the wrapper functions below instead.

## Functions

### `print(s: *u8)`
Writes a null-terminated string to stdout, **without** a trailing newline. Implemented as `c_printf("%s", s)`.

### `println(s: *u8)`
Writes a null-terminated string to stdout, followed by a newline. Implemented via `puts`.

### `print_i64(n: i64)`
Writes a 64-bit signed integer to stdout, no trailing newline (`printf("%d", n)`).

### `println_i64(n: i64)`
Writes a 64-bit signed integer to stdout, followed by a newline (`printf("%d\n", n)`).

### `eprint(s: *u8)`
Writes a null-terminated string to **stderr**, no trailing newline.

### `eprintln(s: *u8)`
Writes a null-terminated string to **stderr**, followed by a newline.

## Example

```marrow
@import("io.mw")

@export fn main(argc: i32, argv: rawptr) -> i32 {
    println("starting up");
    print("argc = ");
    println_i64(cast(i64) argc);
    eprintln("this goes to stderr");
    ret 0;
};
```