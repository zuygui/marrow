---
title: "Text"
description: Character classes, searching, parsing and splitting.
sidebar:
  order: 16
---

Source: `std/text.mw`.

Text utilities on NUL-terminated byte strings (`*u8`) and on `String`.


## Functions

### `is_upper`

```marrow
fn is_upper(c: u8) -> bool
```

### `is_lower`

```marrow
fn is_lower(c: u8) -> bool
```

### `is_alnum`

```marrow
fn is_alnum(c: u8) -> bool
```

### `is_hex_digit`

```marrow
fn is_hex_digit(c: u8) -> bool
```

### `to_lower`

```marrow
fn to_lower(c: u8) -> u8
```

### `to_upper`

```marrow
fn to_upper(c: u8) -> u8
```

### `hex_value`

```marrow
fn hex_value(c: u8) -> i64
```

Value of a hexadecimal digit, or -1.


### `str_cmp`

```marrow
fn str_cmp(a: *u8, b: *u8) -> i32
```

Lexicographic comparison on bytes: negative, zero or positive.


### `str_starts_with`

```marrow
fn str_starts_with(s: *u8, prefix: *u8) -> bool
```

### `str_ends_with`

```marrow
fn str_ends_with(s: *u8, suffix: *u8) -> bool
```

### `str_find_char`

```marrow
fn str_find_char(s: *u8, c: u8) -> i64
```

Index of the first occurrence of `c`, or -1.


### `str_rfind_char`

```marrow
fn str_rfind_char(s: *u8, c: u8) -> i64
```

### `str_find`

```marrow
fn str_find(hay: *u8, needle: *u8) -> i64
```

Index of the first occurrence of `needle` in `hay`, or -1.


### `str_dup`

```marrow
fn str_dup(s: *u8) -> *u8
```

Copy into freshly allocated memory (free it with `dealloc`).


### `str_dup_n`

```marrow
fn str_dup_n(s: *u8, n: i64) -> *u8
```

### `str_concat`

```marrow
fn str_concat(a: *u8, b: *u8) -> *u8
```

### `str_to_lower_in_place`

```marrow
fn str_to_lower_in_place(s: *u8)
```

### `str_to_upper_in_place`

```marrow
fn str_to_upper_in_place(s: *u8)
```

### `str_eq_nocase`

```marrow
fn str_eq_nocase(a: *u8, b: *u8) -> bool
```

Case-insensitive (ASCII) equality.


### `parse_i64`

```marrow
fn parse_i64(s: *u8, out: *i64) -> bool
```

Parses an optionally signed decimal integer; the whole string must be used.


### `parse_u64`

```marrow
fn parse_u64(s: *u8, base: i64, out: *u64) -> bool
```

### `parse_f64`

```marrow
fn parse_f64(s: *u8, out: *f64) -> bool
```

Parses decimal floats such as `-12.5`, `.5`, `3.`, `1e-9`, `2.5E+3`. The whole string must be used. Correctly rounded for ordinary inputs (up to 19 significant digits and exponents within +-22), a few ulps off beyond that.


### `str_split`

```marrow
fn str_split(s: *u8, sep: u8) -> *VecPtr
```

Splits on a separator byte. Returns a vector of freshly allocated C strings (free each with `dealloc`, then the vector with `vec_ptr_free`).


### `str_trim`

```marrow
fn str_trim(s: *u8) -> *u8
```

Trims ASCII whitespace from both ends; returns a new C string.


### `string_push_i64`

```marrow
fn string_push_i64(s: *String, n: i64)
```

### `string_push_u64`

```marrow
fn string_push_u64(s: *String, n: u64)
```

### `string_push_hex`

```marrow
fn string_push_hex(s: *String, n: u64)
```

### `string_push_f64`

```marrow
fn string_push_f64(s: *String, x: f64, decimals: i64)
```

### `string_push_bytes`

```marrow
fn string_push_bytes(s: *String, data: *u8, len: i64)
```

Appends `len` raw bytes (which may contain NULs).


### `string_clear`

```marrow
fn string_clear(s: *String)
```

### `string_dup_cstr`

```marrow
fn string_dup_cstr(s: *String) -> *u8
```

