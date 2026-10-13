---
title: "Rand"
description: Random numbers.
sidebar:
  order: 27
---

Source: `std/rand.mw`.

Seeded from the operating system (`getrandom` or its equivalent).

Random numbers.

- `random_bytes` / `random_u64`: cryptographically secure, straight from the OS.
- `Rng`: fast xoshiro256\*\* generator for simulations, shuffles, tests.

## Types

### `Rng`

```marrow
struct Rng {
    s0: u64;
    s1: u64;
    s2: u64;
    s3: u64;
}
```

## Functions

### `random_bytes`

```marrow
fn random_bytes(buf: *u8, len: i64) -> i64
```

### `random_u64`

```marrow
fn random_u64() -> u64
```

### `rotl64`

```marrow
fn rotl64(x: u64, k: i64) -> u64
```

### `splitmix64`

```marrow
fn splitmix64(state: *u64) -> u64
```

### `rng_seeded`

```marrow
fn rng_seeded(seed: u64) -> Rng
```

### `rng_new`

```marrow
fn rng_new() -> Rng
```

Seeded from the operating system.


### `rng_next`

```marrow
fn rng_next(r: *Rng) -> u64
```

### `rng_below`

```marrow
fn rng_below(r: *Rng, n: u64) -> u64
```

Uniform in [0, n) without modulo bias (rejection sampling). `n` must be positive.


### `rng_range`

```marrow
fn rng_range(r: *Rng, lo: i64, hi: i64) -> i64
```

Uniform in [lo, hi] (inclusive).


### `rng_f64`

```marrow
fn rng_f64(r: *Rng) -> f64
```

Uniform in [0, 1).


### `rng_bool`

```marrow
fn rng_bool(r: *Rng) -> bool
```

