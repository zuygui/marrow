---
title: "IO"
description: Printing to stdout and stderr.
sidebar:
  order: 11
---

Source: `std/io.mw`.

Everything here writes straight to file descriptors 1 and 2 through the platform layer; there is no buffering.

Console output on top of raw write(2): no libc, no buffering.


## Functions

### `print`

```marrow
fn print(s: *u8)
```

### `println`

```marrow
fn println(s: *u8)
```

### `eprint`

```marrow
fn eprint(s: *u8)
```

### `eprintln`

```marrow
fn eprintln(s: *u8)
```

### `print_char`

```marrow
fn print_char(c: u8)
```

### `print_i64`

```marrow
fn print_i64(n: i64)
```

### `println_i64`

```marrow
fn println_i64(n: i64)
```

### `print_u64`

```marrow
fn print_u64(n: u64)
```

### `println_u64`

```marrow
fn println_u64(n: u64)
```

### `print_hex`

```marrow
fn print_hex(n: u64)
```

### `print_f64`

```marrow
fn print_f64(x: f64, decimals: i64)
```

### `println_f64`

```marrow
fn println_f64(x: f64, decimals: i64)
```

