---
title: "string — C-strings and the growable String type"
description: C-string helpers, character classification, and the growable String type from std/string.mw.
sidebar:
  order: 4
---

Source: `std/string.mw`. Imports [`mem.mw`](/standard-library/mem/) (for `alloc`/`realloc`, used by `String`).

## C-string helpers

These operate on plain `*u8` null-terminated strings (the same representation Marrow string *literals* have).

### `str_len(s: *u8) -> i64`
Returns the length of a null-terminated string, not counting the terminator (like C's `strlen`).

### `str_eq(s1: *u8, s2: *u8) -> i8`
Returns `1` if the two null-terminated strings are equal, `0` otherwise.

## Character classification

Each takes a `u8` (a single byte/char) and returns `1`/`0`.

### `is_digit(c: u8) -> i8`
True for ASCII `'0'`–`'9'`.

### `is_alpha(c: u8) -> i8`
True for ASCII letters (`'a'`–`'z'`, `'A'`–`'Z'`) **or underscore** (`'_'`) — despite the name, this also accepts `_`, which makes it convenient for scanning identifier-like text.

### `is_space(c: u8) -> i8`
True for space, tab (`\t`), newline (`\n`) or carriage return (`\r`).

## The `String` type

A growable, heap-allocated, null-terminated byte string — distinct from the built-in `*u8` C-string type, and from `u8[]` slices.

```marrow
struct String {
    data: *u8;   // heap buffer, always null-terminated
    len: i64;    // length, not counting the null terminator
    cap: i64;    // allocated capacity of `data`
}
```

`String` values are always handled through a `*String` pointer — every constructor below returns one, and every function that takes a `String` takes a pointer to it.

### `string_new(initial_cap: i64) -> *String`
Allocates a new, empty `String` with at least `initial_cap` bytes of backing storage (silently bumped up to a minimum of `8` if you ask for less). The buffer starts null-terminated (`len = 0`).

### `string_from(raw_str: *u8) -> *String`
Builds a new `String` by copying the contents of a null-terminated C-string.

### `string_push_char(s: *String, c: u8)`
Appends a single byte, growing (doubling capacity) and `realloc`ing the buffer if needed, and keeps the buffer null-terminated.

### `string_push_str(s: *String, raw_str: *u8)`
Appends the contents of a null-terminated C-string, growing as needed.

### `string_free(s: *String)`
Frees both the string's data buffer and the `String` struct itself. After this call, `s` is a dangling pointer — don't use it again.

### `string_eq(a: *String, b: *String) -> i32`
Equality check. Returns `0` if either pointer is null; `1` if they're the same pointer; otherwise compares lengths first, then byte content.

### `string_cmp(a: *String, b: *String) -> i32`
Lexicographic (byte-wise) comparison, returning `-1`, `0` or `1` — the same convention as C's `strcmp`/`memcmp`, except it also correctly orders strings of different lengths that share a common prefix (the shorter one sorts first). Returns `0` if either pointer, or either pointer's `data`, is null.

## Example

```marrow
@import("string.mw")

fn greet(name: *u8) -> *String {
    var s: *String = string_new(16);
    string_push_str(s, "Hello, ");
    string_push_str(s, name);
    string_push_char(s, cast(u8) 33); // '!'
    ret s;
}
```