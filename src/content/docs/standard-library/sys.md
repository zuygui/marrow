---
title: "Sys"
description: Exit, panic and assert.
sidebar:
  order: 20
---

Source: `std/sys.mw`.

Process-level helpers. `exec` runs a command through `/bin/sh -c` and is only available where the platform layer supports it.

## Functions

### `exit`

```marrow
fn exit(code: i32)
```

### `exec`

```marrow
fn exec(command: *u8) -> i32
```

Runs a shell command and returns its exit status (-1 when unsupported).


### `panic`

```marrow
fn panic(message: *u8)
```

### `assert`

```marrow
fn assert(condition: i8, message: *u8)
```

### `getpid`

```marrow
fn getpid() -> i64
```

