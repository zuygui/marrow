---
title: "Math"
description: Floating-point and integer math, written from scratch.
sidebar:
  order: 26
---

Source: `std/math.mw`.

Everything is implemented in Marrow (no libm): `sqrt`, `exp`, `ln`, `pow`, the trigonometric functions and their inverses are accurate to a few ulps, and edge cases (NaN, infinities, signed zeros) follow IEEE 754.

Math library in plain Marrow (no libm). Results are accurate to a few ulps for ordinary arguments. Trigonometric functions reduce their argument with a three-part constant (Cody-Waite), which is accurate for |x| below about 1e6; larger arguments still give a result, with fewer correct digits.


## Constants

| Name | Type | Value | Notes |
|---|---|---|---|
| `PI` | `f64` | `3.141592653589793` |  |
| `TAU` | `f64` | `6.283185307179586` |  |
| `HALF_PI` | `f64` | `1.5707963267948966` |  |
| `E` | `f64` | `2.718281828459045` |  |
| `LN2` | `f64` | `0.6931471805599453` |  |
| `LN10` | `f64` | `2.302585092994046` |  |
| `SQRT2` | `f64` | `1.4142135623730951` |  |

## Functions

### `f64_to_bits`

```marrow
fn f64_to_bits(x: f64) -> u64
```

### `f64_from_bits`

```marrow
fn f64_from_bits(b: u64) -> f64
```

### `math_nan`

```marrow
fn math_nan() -> f64
```

### `math_inf`

```marrow
fn math_inf() -> f64
```

### `is_nan`

```marrow
fn is_nan(x: f64) -> bool
```

### `is_inf`

```marrow
fn is_inf(x: f64) -> bool
```

### `is_finite`

```marrow
fn is_finite(x: f64) -> bool
```

### `signbit`

```marrow
fn signbit(x: f64) -> bool
```

### `abs_i64`

```marrow
fn abs_i64(x: i64) -> i64
```

### `min_i64`

```marrow
fn min_i64(a: i64, b: i64) -> i64
```

### `max_i64`

```marrow
fn max_i64(a: i64, b: i64) -> i64
```

### `clamp_i64`

```marrow
fn clamp_i64(x: i64, lo: i64, hi: i64) -> i64
```

### `gcd_i64`

```marrow
fn gcd_i64(a: i64, b: i64) -> i64
```

### `pow_i64`

```marrow
fn pow_i64(base: i64, exp: i64) -> i64
```

Integer power by repeated squaring (wraps on overflow).


### `isqrt_u64`

```marrow
fn isqrt_u64(n: u64) -> u64
```

Largest r with r \* r &lt;= n (0 for negative n).


### `abs`

```marrow
fn abs(x: f64) -> f64
```

### `copysign`

```marrow
fn copysign(x: f64, sign: f64) -> f64
```

### `min`

```marrow
fn min(a: f64, b: f64) -> f64
```

### `max`

```marrow
fn max(a: f64, b: f64) -> f64
```

### `clamp`

```marrow
fn clamp(x: f64, lo: f64, hi: f64) -> f64
```

### `trunc`

```marrow
fn trunc(x: f64) -> f64
```

### `floor`

```marrow
fn floor(x: f64) -> f64
```

### `ceil`

```marrow
fn ceil(x: f64) -> f64
```

### `round`

```marrow
fn round(x: f64) -> f64
```

Rounds half away from zero.


### `fract`

```marrow
fn fract(x: f64) -> f64
```

### `fmod`

```marrow
fn fmod(x: f64, y: f64) -> f64
```

Remainder with the sign of `x` (like C's fmod).


### `sqrt`

```marrow
fn sqrt(x: f64) -> f64
```

### `cbrt`

```marrow
fn cbrt(x: f64) -> f64
```

### `hypot`

```marrow
fn hypot(x: f64, y: f64) -> f64
```

### `exp`

```marrow
fn exp(x: f64) -> f64
```

### `ln`

```marrow
fn ln(x: f64) -> f64
```

### `log2`

```marrow
fn log2(x: f64) -> f64
```

### `log10`

```marrow
fn log10(x: f64) -> f64
```

### `pow`

```marrow
fn pow(x: f64, y: f64) -> f64
```

### `sinh`

```marrow
fn sinh(x: f64) -> f64
```

### `cosh`

```marrow
fn cosh(x: f64) -> f64
```

### `tanh`

```marrow
fn tanh(x: f64) -> f64
```

### `sin`

```marrow
fn sin(x: f64) -> f64
```

### `cos`

```marrow
fn cos(x: f64) -> f64
```

### `tan`

```marrow
fn tan(x: f64) -> f64
```

### `atan`

```marrow
fn atan(x: f64) -> f64
```

### `atan2`

```marrow
fn atan2(y: f64, x: f64) -> f64
```

### `asin`

```marrow
fn asin(x: f64) -> f64
```

### `acos`

```marrow
fn acos(x: f64) -> f64
```

### `to_radians`

```marrow
fn to_radians(deg: f64) -> f64
```

### `to_degrees`

```marrow
fn to_degrees(rad: f64) -> f64
```

