---
title: "AES-GCM"
description: AES-128-GCM.
sidebar:
  order: 35
---

Source: `std/aes.mw`.

AES-128 and the AES-128-GCM AEAD (NIST SP 800-38D).  Table-driven software implementation (S-box built at first use); keys are 16 bytes, nonces 12 bytes, tags 16 bytes.  Like any table-based AES it is not constant time with respect to cache timing.


## Constants

| Name | Type | Value | Notes |
|---|---|---|---|
| `AES_SBOX` | `*u8` | `null` |  |

## Functions

### `aes_init`

```marrow
fn aes_init()
```

### `aes128_expand`

```marrow
fn aes128_expand(key: *u8, rk: *u8)
```

Expands a 16-byte key into 176 bytes of round keys.


### `aes128_encrypt_block`

```marrow
fn aes128_encrypt_block(rk: *u8, in: *u8, out: *u8)
```

### `aes128gcm_seal`

```marrow
fn aes128gcm_seal(key: *u8, nonce: *u8, aad: *u8, aadlen: i64, pt: *u8, len: i64, out: *u8) -> i64
```

Writes ciphertext followed by the 16-byte tag; returns len + 16.


### `aes128gcm_open`

```marrow
fn aes128gcm_open(key: *u8, nonce: *u8, aad: *u8, aadlen: i64, ct: *u8, len: i64, out: *u8) -> i64
```

`ct` is ciphertext||tag (len bytes in total).  Returns the plaintext length or -1 if authentication fails (nothing is written to `out` then).


