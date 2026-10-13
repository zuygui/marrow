---
title: "Encoding"
description: Hex, Base64 and UTF-8.
sidebar:
  order: 41
---

Source: `std/encoding.mw`.

Hex, Base64 (standard and URL-safe) and UTF-8 validation and conversion.

Hex, Base64 (RFC 4648, standard and URL-safe) and UTF-8 helpers. Encoders write into a caller buffer and return the length; decoders return the decoded length or -1 on malformed input.  `*_len` helpers give the worst-case output size.


## Functions

### `hex_encode_len`

```marrow
fn hex_encode_len(n: i64) -> i64
```

### `hex_decode_len`

```marrow
fn hex_decode_len(n: i64) -> i64
```

### `hex_encode`

```marrow
fn hex_encode(src: *u8, n: i64, out: *u8) -> i64
```

### `hex_decode`

```marrow
fn hex_decode(src: *u8, n: i64, out: *u8) -> i64
```

### `base64_encode_len`

```marrow
fn base64_encode_len(n: i64) -> i64
```

### `base64_decode_len`

```marrow
fn base64_decode_len(n: i64) -> i64
```

### `base64_encode`

```marrow
fn base64_encode(src: *u8, n: i64, out: *u8) -> i64
```

### `base64url_encode`

```marrow
fn base64url_encode(src: *u8, n: i64, out: *u8) -> i64
```

URL-safe alphabet, no padding (as used by JWT).


### `base64_decode`

```marrow
fn base64_decode(src: *u8, n: i64, out: *u8) -> i64
```

Accepts both alphabets, optional padding, and skips ASCII whitespace (so PEM bodies can be decoded directly).


### `utf8_valid`

```marrow
fn utf8_valid(s: *u8, n: i64) -> bool
```

### `utf8_count`

```marrow
fn utf8_count(s: *u8, n: i64) -> i64
```

### `utf8_decode`

```marrow
fn utf8_decode(s: *u8, n: i64, len: *i64) -> i64
```

Decodes one code point; stores its byte length in \*len.  Returns -1 if malformed.


### `utf8_encode`

```marrow
fn utf8_encode(cp: i64, out: *u8) -> i64
```

Encodes a code point into `out` (up to 4 bytes); returns the length, 0 if invalid.


