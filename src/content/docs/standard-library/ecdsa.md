---
title: "ECDSA"
description: ECDSA signature verification on P-256 and P-384.
sidebar:
  order: 39
---

Source: `std/ecdsa.mw`.

Signature verification only, on NIST P-256 and P-384.

ECDSA signature verification over NIST P-256 and P-384 (FIPS 186-4), with Jacobian-coordinate arithmetic and Shamir's trick.  Verification only: all inputs are public, so the variable-time bigint code is acceptable.


## Constants

| Name | Type | Value | Notes |
|---|---|---|---|
| `ECC_P256` | `i64` | `1` |  |
| `ECC_P384` | `i64` | `2` |  |
| `EC_POOL` | `i64` | `0` | ---- temporaries: every pooled Big is released in one go ---- |
| `EC_POOL_N` | `i64` | `0` |  |
| `EC_POOL_CAP` | `i64` | `0` |  |

## Functions

### `ecdsa_verify`

```marrow
fn ecdsa_verify(curve: i64, pub: *u8, publen: i64, hash: *u8, hlen: i64, rb: *u8, rlen: i64, sb: *u8, slen: i64) -> bool
```

Verifies (r, s) over `hash` against the uncompressed public key `pub` (04 || X || Y).  r and s are big-endian magnitudes.


### `ecdsa_verify_der`

```marrow
fn ecdsa_verify_der(curve: i64, pub: *u8, publen: i64, hash: *u8, hlen: i64, sig: *u8, siglen: i64) -> bool
```

Same, with the signature as DER SEQUENCE { INTEGER r, INTEGER s }.


