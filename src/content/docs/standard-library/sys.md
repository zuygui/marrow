---
title: "sys — process control"
description: exit, shelling out, panic and assert from std/sys.mw.
sidebar:
  order: 8
---

Source: `std/sys.mw`. Imports [`io.mw`](/standard-library/io/).

## Foreign bindings

| Function | Binds to (C) |
|---|---|
| `exit(code: i32)` | `exit` |
| `c_system(command: *u8) -> i32` | `system` |

`exit` is exposed directly under its C name — there's no Marrow-native wrapper, since the raw `exit(3)` signature is already exactly what you want.

## Functions

### `exec(command: *u8) -> i32`
Runs a shell command via `system(3)` and returns its exit status. Blocks until the command finishes, exactly like C's `system()` (including running through `/bin/sh -c` on Unix-like systems).

### `panic(message: *u8)`
Prints `[FATAL PANIC] <message>` to stderr and immediately terminates the process with `exit(1)`. This function never returns — control flow does not continue past a `panic()` call, even though the compiler doesn't currently model "never-returning" functions specially (the process is simply gone by the time execution would resume).

### `assert(condition: i8, message: *u8)`
If `condition` is `0` (falsy), calls `panic(message)`; otherwise does nothing. This is the standard library's only assertion mechanism — there's no built-in `assert` keyword or macro, just this plain function.

## Example

```marrow
@import("sys.mw")

fn divide(a: i64, b: i64) -> i64 {
    assert(cast(i8) (b != 0), "division by zero");
    ret a / b;
}

fn main(argc: i32, argv: rawptr) -> i32 {
    if (argc < 2) {
        panic("usage: prog <arg>");
    }
    ret exec("echo running");
};
```
