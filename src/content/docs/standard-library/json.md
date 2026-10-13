---
title: "JSON"
description: A JSON parser and writer.
sidebar:
  order: 42
---

Source: `std/json.mw`.

Parses text into a tree of `Json` values and writes it back, with optional pretty-printing. On a parse error `json_parse` returns null; `json_error` and `json_error_pos` say why and where.

JSON (RFC 8259): a strict parser producing a tree of \*Json values, a builder API and a serializer.

```marrow
var v: *Json = json_parse(text, len);          // null on error
if v == null { println(json_error()); }
var name: *Json = json_get(v, "name");
println(json_as_str(name));
var out: *u8 = json_stringify(v, true);         // pretty printed
json_free(v);
```

Parsed integers that fit an i64 keep full precision (json\_as\_i64); everything else is a double.  Builder functions take ownership of the values passed in.


## Constants

| Name | Type | Value | Notes |
|---|---|---|---|
| `JSON_NULL` | `i64` | `0` |  |
| `JSON_BOOL` | `i64` | `1` |  |
| `JSON_NUMBER` | `i64` | `2` |  |
| `JSON_STRING` | `i64` | `3` |  |
| `JSON_ARRAY` | `i64` | `4` |  |
| `JSON_OBJECT` | `i64` | `5` |  |

## Types

### `Json`

```marrow
struct Json {
    kind: i64;
    flag: i64;          // bool value, or 1 when the number is an exact integer
    ival: i64;
    num: f64;
    str: *u8;           // NUL-terminated copy of the string value
    slen: i64;
    items: *VecPtr;     // array elements / object values
    keys: *VecPtr;      // object keys (*u8)
}
```

## Functions

### `json_error`

```marrow
fn json_error() -> *u8
```

Reason for the last failed json\_parse, and the byte offset where it happened.


### `json_error_pos`

```marrow
fn json_error_pos() -> i64
```

### `json_new`

```marrow
fn json_new(kind: i64) -> *Json
```

### `json_null`

```marrow
fn json_null() -> *Json
```

### `json_bool`

```marrow
fn json_bool(b: bool) -> *Json
```

### `json_int`

```marrow
fn json_int(n: i64) -> *Json
```

### `json_num`

```marrow
fn json_num(x: f64) -> *Json
```

### `json_str_n`

```marrow
fn json_str_n(s: *u8, len: i64) -> *Json
```

A string value from `len` bytes (may contain NUL).


### `json_str`

```marrow
fn json_str(s: *u8) -> *Json
```

### `json_array`

```marrow
fn json_array() -> *Json
```

### `json_object`

```marrow
fn json_object() -> *Json
```

### `json_free`

```marrow
fn json_free(v: *Json)
```

### `json_kind`

```marrow
fn json_kind(v: *Json) -> i64
```

### `json_is_null`

```marrow
fn json_is_null(v: *Json) -> bool
```

### `json_len`

```marrow
fn json_len(v: *Json) -> i64
```

### `json_at`

```marrow
fn json_at(v: *Json, i: i64) -> *Json
```

### `json_key_at`

```marrow
fn json_key_at(v: *Json, i: i64) -> *u8
```

### `json_value_at`

```marrow
fn json_value_at(v: *Json, i: i64) -> *Json
```

### `json_index_of`

```marrow
fn json_index_of(v: *Json, key: *u8) -> i64
```

### `json_get`

```marrow
fn json_get(v: *Json, key: *u8) -> *Json
```

Member of an object, or null.


### `json_has`

```marrow
fn json_has(v: *Json, key: *u8) -> bool
```

### `json_as_str`

```marrow
fn json_as_str(v: *Json) -> *u8
```

### `json_as_bool`

```marrow
fn json_as_bool(v: *Json) -> bool
```

### `json_as_f64`

```marrow
fn json_as_f64(v: *Json) -> f64
```

### `json_as_i64`

```marrow
fn json_as_i64(v: *Json) -> i64
```

Integer value; fractional numbers are truncated toward zero.


### `json_push`

```marrow
fn json_push(arr: *Json, item: *Json)
```

Appends `item` (ownership moves to the array).


### `json_set`

```marrow
fn json_set(obj: *Json, key: *u8, val: *Json)
```

Sets a member (replacing an existing one); ownership of `val` moves to the object.


### `json_parse`

```marrow
fn json_parse(text: *u8, len: i64) -> *Json
```

Parses `len` bytes of JSON text; null (see json\_error) on any syntax error or trailing garbage.


### `json_parse_str`

```marrow
fn json_parse_str(text: *u8) -> *Json
```

Convenience for NUL-terminated text.


### `json_stringify`

```marrow
fn json_stringify(v: *Json, pretty: bool) -> *u8
```

Serializes a value into a new NUL-terminated string (release with dealloc). `pretty` uses two-space indentation.


