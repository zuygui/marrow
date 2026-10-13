---
title: "String"
description: C-string helpers and the growable `String`.
sidebar:
  order: 15
---

Source: `std/string.mw`.

C strings are plain `*u8` values ending in a zero byte. `String` is the growable owned variant.

## Types

### `String`

```marrow
struct String {
    data: *u8;
    len: i64;
    cap: i64;
}
```

## Functions

### `str_len`

```marrow
fn str_len(s: *u8) -> i64
```

### `str_eq`

```marrow
fn str_eq(s1: *u8, s2: *u8) -> i8
```

### `is_digit`

```marrow
fn is_digit(c: u8) -> i8
```

### `is_alpha`

```marrow
fn is_alpha(c: u8) -> i8
```

### `is_space`

```marrow
fn is_space(c: u8) -> i8
```

### `string_new`

```marrow
fn string_new(initial_cap: i64) -> *String
```

### `string_from`

```marrow
fn string_from(raw_str: *u8) -> *String
```

### `string_push_char`

```marrow
fn string_push_char(s: *String, c: u8)
```

### `string_push_str`

```marrow
fn string_push_str(s: *String, raw_str: *u8)
```

### `string_free`

```marrow
fn string_free(s: *String)
```

### `string_eq`

```marrow
fn string_eq(a: *String, b: *String) -> i32
```

### `string_cmp`

```marrow
fn string_cmp(a: *String, b: *String) -> i32
```

