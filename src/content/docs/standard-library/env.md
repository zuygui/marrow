---
title: "Env"
description: Command-line arguments and environment variables.
sidebar:
  order: 22
---

Source: `std/env.mw`.

Command-line arguments and environment variables.  Call env\_init from main first:

```marrow
@export fn main(argc: i32, argv: rawptr) -> i32 {
    env_init(argc, argv);
    ... arg_get(1) ... env_get("HOME") ...
```

The environment block is read from the stack layout every System V style entry point provides (envp follows argv), so this works on Linux and macOS without libc.  Windows keeps its environment elsewhere: there env\_get returns null.


## Constants

| Name | Type | Value | Notes |
|---|---|---|---|
| `ENV_ARGC` | `i64` | `0` |  |
| `ENV_ARGV` | `i64` | `0` |  |
| `ENV_OK` | `i64` | `0` |  |

## Functions

### `env_init`

```marrow
fn env_init(argc: i32, argv: rawptr)
```

### `arg_count`

```marrow
fn arg_count() -> i64
```

### `arg_get`

```marrow
fn arg_get(i: i64) -> *u8
```

The i-th argument (0 is the program name), or null when out of range.


### `env_block`

```marrow
fn env_block() -> *i64
```

### `env_count`

```marrow
fn env_count() -> i64
```

Number of "NAME=value" entries.


### `env_at`

```marrow
fn env_at(i: i64) -> *u8
```

The i-th "NAME=value" entry, or null.


### `env_get`

```marrow
fn env_get(name: *u8) -> *u8
```

Value of the variable (a pointer into the environment block), or null.


### `env_get_or`

```marrow
fn env_get_or(name: *u8, fallback: *u8) -> *u8
```

Like env\_get but returns `fallback` when the variable is not set.


