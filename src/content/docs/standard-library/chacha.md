---
title: "ChaCha20-Poly1305"
description: The ChaCha20-Poly1305 AEAD.
sidebar:
  order: 34
---

Source: `std/chacha.mw`.

ChaCha20, Poly1305 and the ChaCha20-Poly1305 AEAD (RFC 8439). Keys are 32 bytes, nonces 12 bytes, tags 16 bytes.


## Functions

### `chacha20_block`

```marrow
fn chacha20_block(key: *u8, nonce: *u8, counter: u32, out: *u8)
```

One 64-byte keystream block.


### `chacha20_xor`

```marrow
fn chacha20_xor(key: *u8, nonce: *u8, counter: u32, in: *u8, out: *u8, len: i64)
```

XORs `len` bytes of `in` with the keystream starting at block `counter`. `in` and `out` may be the same buffer.


### `poly1305`

```marrow
fn poly1305(key: *u8, msg: *u8, len: i64, tag: *u8)
```

### `chacha20poly1305_seal`

```marrow
fn chacha20poly1305_seal(key: *u8, nonce: *u8, aad: *u8, aadlen: i64, pt: *u8, len: i64, out: *u8) -> i64
```

Writes ciphertext followed by the 16-byte tag to `out`; returns len + 16.


### `chacha20poly1305_open`

```marrow
fn chacha20poly1305_open(key: *u8, nonce: *u8, aad: *u8, aadlen: i64, ct: *u8, len: i64, out: *u8) -> i64
```

`ct` holds ciphertext||tag (`len` bytes in total).  Returns the plaintext length, or -1 if authentication fails (nothing is written to `out` then).


