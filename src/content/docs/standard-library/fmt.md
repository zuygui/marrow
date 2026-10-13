---
title: "Fmt"
description: Number formatting into buffers.
sidebar:
  order: 17
---

Source: `std/fmt.mw`.

Number formatting, written in Marrow (no libc `printf`). Every function writes into a caller-provided buffer and returns the number of bytes written; no NUL terminator is added. 64 bytes are always enough.


## Functions

### `fmt_reverse`

```marrow
fn fmt_reverse(buf: *u8, len: i64)
```

### `fmt_u64`

```marrow
fn fmt_u64(buf: *u8, n: u64, base: i64) -> i64
```

Unsigned integer in the given base (2..16), lowercase digits.


### `fmt_i64`

```marrow
fn fmt_i64(buf: *u8, n: i64) -> i64
```

### `fmt_pow10`

```marrow
fn fmt_pow10(n: i64) -> u64
```

### `fmt_copy_str`

```marrow
fn fmt_copy_str(buf: *u8, s: *u8) -> i64
```

### `fmt_f64`

```marrow
fn fmt_f64(buf: *u8, x: f64, decimals: i64) -> i64
```

Fixed-point notation with `decimals` digits after the point (0..15). Values of 1e15 and above use scientific notation (d.dddddde+XX).


