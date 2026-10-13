---
title: "RSA"
description: RSA signature verification.
sidebar:
  order: 38
---

Source: `std/rsa.mw`.

Signature verification only (PKCS#1 v1.5 and PSS), with a public exponent and modulus passed as big-endian bytes.

RSA signature verification: PKCS#1 v1.5 (RFC 8017 §8.2) and PSS (§8.1). `n` and `e` are big-endian magnitudes.  `mhash` is the already computed digest of the message, produced with hash algorithm `alg` (see hash.mw).


## Functions

### `rsa_public_op`

```marrow
fn rsa_public_op(n: *u8, nlen: i64, e: *u8, elen: i64, sig: *u8, siglen: i64, em: *u8) -> bool
```

Computes sig^e mod n into `em` (exactly k = byte length of n).


### `rsa_modulus_len`

```marrow
fn rsa_modulus_len(n: *u8, nlen: i64) -> i64
```

### `rsa_verify_pkcs1`

```marrow
fn rsa_verify_pkcs1(n: *u8, nlen: i64, e: *u8, elen: i64, alg: i64, mhash: *u8, sig: *u8, siglen: i64) -> bool
```

### `rsa_verify_pss`

```marrow
fn rsa_verify_pss(n: *u8, nlen: i64, e: *u8, elen: i64, alg: i64, mhash: *u8, saltlen: i64, sig: *u8, siglen: i64) -> bool
```

RSASSA-PSS with MGF1 using the same hash, salt length `saltlen` (TLS 1.3 uses saltlen = hash length).


