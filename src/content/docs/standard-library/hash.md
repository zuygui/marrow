---
title: "Hash"
description: SHA-1/2, HMAC, HKDF and PBKDF2.
sidebar:
  order: 33
---

Source: `std/hash.mw`.

Algorithms are selected with an integer id. All of this code is written for verification and key derivation; it is not hardened against timing side channels except `ct_equal`.

Cryptographic hashes (SHA-1, SHA-256, SHA-384, SHA-512), HMAC, HKDF and PBKDF2.  Everything is one-shot: pass the whole message and an output buffer of `hash_size(alg)` bytes.  No libc, no function pointers.


## Constants

| Name | Type | Value | Notes |
|---|---|---|---|
| `HASH_SHA1` | `i64` | `1` |  |
| `HASH_SHA256` | `i64` | `2` |  |
| `HASH_SHA384` | `i64` | `3` |  |
| `HASH_SHA512` | `i64` | `4` |  |

## Functions

### `hash_size`

```marrow
fn hash_size(alg: i64) -> i64
```

### `hash_block_size`

```marrow
fn hash_block_size(alg: i64) -> i64
```

### `sha256`

```marrow
fn sha256(data: *u8, len: i64, out: *u8)
```

### `sha512`

```marrow
fn sha512(data: *u8, len: i64, out: *u8)
```

### `sha384`

```marrow
fn sha384(data: *u8, len: i64, out: *u8)
```

### `sha1`

```marrow
fn sha1(data: *u8, len: i64, out: *u8)
```

### `hash_digest`

```marrow
fn hash_digest(alg: i64, data: *u8, len: i64, out: *u8) -> i64
```

Writes hash\_size(alg) bytes to `out`; returns that size (0 for unknown alg).


### `hmac`

```marrow
fn hmac(alg: i64, key: *u8, klen: i64, msg: *u8, mlen: i64, out: *u8) -> i64
```

### `hkdf_extract`

```marrow
fn hkdf_extract(alg: i64, salt: *u8, slen: i64, ikm: *u8, ilen: i64, prk: *u8) -> i64
```

HKDF (RFC 5869).  `out` receives `olen` bytes (olen &lt;= 255 \* hash\_size).


### `hkdf_expand`

```marrow
fn hkdf_expand(alg: i64, prk: *u8, plen: i64, info: *u8, infolen: i64, out: *u8, olen: i64) -> bool
```

### `pbkdf2`

```marrow
fn pbkdf2(alg: i64, pass: *u8, plen: i64, salt: *u8, slen: i64, iters: i64, out: *u8, olen: i64) -> bool
```

PBKDF2-HMAC (RFC 8018).


### `ct_equal`

```marrow
fn ct_equal(a: *u8, b: *u8, n: i64) -> bool
```

Constant-time comparison.


