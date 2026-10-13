---
title: "BigInt"
description: Arbitrary-precision unsigned integers.
sidebar:
  order: 37
---

Source: `std/bigint.mw`.

Unsigned big integers on 32-bit limbs, enough for RSA and the elliptic curves used by TLS. Operations allocate their result.

Arbitrary-precision unsigned integers (little-endian 32-bit limbs) with the modular arithmetic needed for RSA and elliptic curves.  Every function that returns a \*Big returns a fresh value the caller releases with big\_free. Reduction is shift-and-subtract: simple and constant-memory, adequate for signature \*verification\* (public data); it is not constant time.


## Types

### `Big`

```marrow
struct Big {
    d: *u32;
    n: i64;
}
```

## Functions

### `big_alloc`

```marrow
fn big_alloc(n: i64) -> *Big
```

### `big_free`

```marrow
fn big_free(b: *Big)
```

### `big_trim`

```marrow
fn big_trim(b: *Big)
```

### `big_from_u64`

```marrow
fn big_from_u64(v: u64) -> *Big
```

### `big_copy`

```marrow
fn big_copy(a: *Big) -> *Big
```

### `big_from_bytes`

```marrow
fn big_from_bytes(p: *u8, len: i64) -> *Big
```

### `big_to_bytes`

```marrow
fn big_to_bytes(a: *Big, out: *u8, len: i64) -> bool
```

Writes `a` big-endian, left-padded with zeros to exactly `len` bytes. Returns false if it does not fit.


### `big_is_zero`

```marrow
fn big_is_zero(a: *Big) -> bool
```

### `big_bits`

```marrow
fn big_bits(a: *Big) -> i64
```

### `big_bit`

```marrow
fn big_bit(a: *Big, i: i64) -> i64
```

### `big_is_odd`

```marrow
fn big_is_odd(a: *Big) -> bool
```

### `big_cmp`

```marrow
fn big_cmp(a: *Big, b: *Big) -> i64
```

### `big_add`

```marrow
fn big_add(a: *Big, b: *Big) -> *Big
```

### `big_sub`

```marrow
fn big_sub(a: *Big, b: *Big) -> *Big
```

a - b, requires a >= b.


### `big_mul`

```marrow
fn big_mul(a: *Big, b: *Big) -> *Big
```

### `big_mod`

```marrow
fn big_mod(a: *Big, m: *Big) -> *Big
```

a mod m (m > 0).


### `big_modmul`

```marrow
fn big_modmul(a: *Big, b: *Big, m: *Big) -> *Big
```

### `big_modadd`

```marrow
fn big_modadd(a: *Big, b: *Big, m: *Big) -> *Big
```

### `big_modsub`

```marrow
fn big_modsub(a: *Big, b: *Big, m: *Big) -> *Big
```

(a - b) mod m for a, b &lt; m.


### `big_modexp`

```marrow
fn big_modexp(base: *Big, exp: *Big, m: *Big) -> *Big
```

### `big_modinv_prime`

```marrow
fn big_modinv_prime(a: *Big, p: *Big) -> *Big
```

a^-1 mod p for prime p (Fermat).


