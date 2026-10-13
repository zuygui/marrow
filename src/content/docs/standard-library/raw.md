---
title: "Raw memory"
description: Byte-level copy, fill and compare primitives.
sidebar:
  order: 14
---

Source: `std/raw.mw`.

Memory primitives written in plain Marrow, no libc.


## Functions

### `raw_copy`

```marrow
fn raw_copy(dest: *u8, src: *u8, count: i64)
```

Copies `count` bytes from `src` to `dest`. The regions must not overlap.


### `raw_move`

```marrow
fn raw_move(dest: *u8, src: *u8, count: i64)
```

Like `raw_copy`, but the regions may overlap.


### `raw_fill`

```marrow
fn raw_fill(dest: *u8, value: u8, count: i64)
```

### `raw_zero`

```marrow
fn raw_zero(dest: *u8, count: i64)
```

### `raw_compare`

```marrow
fn raw_compare(a: *u8, b: *u8, count: i64) -> i32
```

Returns a negative, zero or positive value like C's memcmp.


