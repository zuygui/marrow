---
title: Variadic functions
description: "The ... parameter marker and the va_start / va_arg / va_end built-ins."
sidebar:
  order: 7
---

Marrow supports C-ABI-compatible variadic functions, both for **declaring** them (mostly to bind to C functions like `printf`) and for **consuming** variadic arguments inside a Marrow function body.

## Declaring & calling a variadic function

```marrow
@extern("printf") fn c_printf(fmt: *u8, val: rawptr) -> i32;
```

A trailing `...` in the parameter list marks a function as variadic:

```marrow
fn sum_all(count: i64, ...) -> i64 { ... }
```

When calling a variadic function, arguments beyond the fixed parameters go through **default argument promotion**, matching C's rules:

- `i8`, `i16`, `bool` → promoted to `i32`
- `u8`, `u16` → promoted to `u32`
- `f32` → promoted to `f64`
- Everything else (`i32`/`u32`/`i64`/`u64`/`f64`/pointers) is passed as-is.

Struct, array and slice arguments **cannot** be passed through the variable part of a call — pass an explicit pointer instead.

## Reading variadic arguments (`va_start` / `va_arg` / `va_end`)

Inside the body of a variadic function, three built-in pseudo-expressions drive the platform's variadic-argument mechanism (they compile directly to QBE's `vastart`/`vaarg` instructions, so they follow your target platform's C ABI):

```marrow
fn my_printf(fmt: *u8, ...) {
    var args = va_start();

    var n: i64 = va_arg(args, i64);
    var f: f64 = va_arg(args, f64);

    va_end(args);
}
```

- **`va_start()`** — no arguments. Allocates and initializes a `va_list`-equivalent buffer, returning it as a `rawptr`. Call this once, before reading any variadic argument.
- **`va_arg(list, Type)`** — reads and consumes the next variadic argument as `Type`, advancing `list`. `Type` **must** be a scalar (integer, float or pointer) — structs, arrays and slices cannot be read this way (QBE itself doesn't support it).
  - Requesting a *narrow* type is rejected at compile time with an explicit error, because it would silently be wrong: since arguments are promoted when passed (see above), you must request `i32`/`u32` instead of `i8`/`i16`/`u8`/`u16`/`bool`, and `f64` instead of `f32`.
- **`va_end(list)`** — signals that you're done reading from `list`. Currently a no-op at the machine level beyond evaluating its argument, but call it for correctness/portability.

## Limitations

- There's no `va_copy`-equivalent.
- No way to know at runtime how many variadic arguments were actually passed — the standard C pattern applies: pass an explicit count or use a sentinel value as one of the fixed parameters (like `printf`'s format string encodes it).