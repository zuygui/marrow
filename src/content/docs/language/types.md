---
title: Types
description: The built-in scalar types, pointers, static arrays, slices and structs, and how they're sized and laid out.
sidebar:
  order: 2
---

Marrow is statically typed, with a small, fixed set of built-in types plus user-defined `struct`s. There is no type inference for function signatures or struct fields — only local variables and global constants can omit an explicit type when it's inferable from the initializer.

## Scalar types

| Type | Size | Notes |
|---|---|---|
| `i8` `i16` `i32` `i64` | 1 / 2 / 4 / 8 bytes | Signed integers. |
| `u8` `u16` `u32` `u64` | 1 / 2 / 4 / 8 bytes | Unsigned integers. |
| `f32` `f64` | 4 / 8 bytes | IEEE-754 floats. |
| `bool` | 1 byte | `true` / `false`. Represented the same way as `u8` at the machine level. |
| `rawptr` | 8 bytes | An untyped pointer (like C's `void*`). |

### Implicit promotion & mixed-type arithmetic

When two operands of different scalar types meet in a binary expression (`+ - * / % == != < <= > >=`), Marrow applies C-like promotion rules:

1. Types smaller than 32 bits (`i8`, `i16`, `u8`, `u16`, `bool`) are first promoted to `i32`/`u32`.
2. If either operand is a float, the result is `f64` if either side is `f64`, otherwise `f32`.
3. Otherwise, if either side is 64-bit, the result is `i64` (or `u64` if either side is unsigned).
4. Otherwise the result is `i32` (or `u32` if either side is unsigned).

Both operands are converted to that common type before the operation, and comparisons (`== != < <= > >=`) always produce a `bool`.

**Pointers behave as unsigned 64-bit integers** in this system: `Pointer(_)` types report 64 bits and are treated as unsigned, so `pointer + integer` performs plain **byte-offset arithmetic** — it is *not* scaled by the pointee's size the way C pointer arithmetic is. This is exactly how the standard library implements things like string/vector growth (`s.data + s.len`, `v.data + (index * 8)`) — the size multiplication, when needed, is written out explicitly.

## Pointers

```marrow
var p: *i64;       // pointer to i64
var pp: **i64;     // pointer to pointer to i64
```

- Written as a prefix `*` before the pointee type.
- `&expr` takes the address of an lvalue (a variable, a dereference, an index, or a struct member) and produces a pointer to its type.
- `*expr` dereferences a pointer, both as a value (`var x = *p;`) and as an assignment target (`*p = 5;`).
- `rawptr` is untyped — think of it as `*void`. It's what most standard-library allocation functions (`alloc`, `dealloc`, `c_malloc`, ...) return/accept when the pointee type isn't meaningful yet.

## Static arrays

```marrow
var buf: [16]u8;      // an array of 16 bytes, allocated inline
var grid: [4][4]f32;  // a 4x4 array of floats
```

- Written as `[N]T`, where `N` is a **compile-time constant integer expression** (integer literals combined with `+ - * /`; no variables, no function calls).
- Arrays are stored inline (by value), not as a pointer — assigning one to a local variable copies the whole array.
- Indexing (`arr[i]`) computes `base_address + i * sizeof(T)`.
- `arr[a..b]` produces a **slice** into the array (see below); `arr[..]`-style full slices need an explicit range — `arr[0..N]` — since arrays don't carry a stored length at runtime to default from at the value level (a static array's length *is* known at compile time from its type, but only the slicing code path currently accepts an implicit end bound derived from that constant `N`, e.g. `arr[2..]` is valid; `arr[..]`/`arr[..]`-without-any-bound alone is not — provide at least a start or use `arr[0..]`).

## Slices

```marrow
var s: i64[];    // a slice of i64
```

- Written as a **postfix** `[]` after the element type (unlike static arrays, which prefix the size).
- At runtime, a slice is represented as a 16-byte, 8-byte-aligned pair: `{ ptr: rawptr, len: i64 }` (pointer first, length second, at offset 8).
- Two fields are accessible by name: `s.ptr` (typed as a pointer to the element type) and `s.len` (an `i64`). Both are also assignable (`s.len = 3;`).
- A slice is produced by slicing an array, another slice, or a pointer:

  ```marrow
  var full: i64[] = arr[..];       // whole array, when a bound can be inferred
  var part: i64[] = arr[2..5];     // elements [2, 5)
  var from2: i64[] = arr[2..];     // elements [2, end)
  var upto5: i64[] = arr[..5];     // elements [0, 5)
  ```

  Slicing a raw pointer (`RType::Pointer`) requires **both** an explicit start and end bound — a pointer alone has no known length to default from.
- Indexing a slice (`s[i]`) loads its `.ptr` field and computes `ptr + i * sizeof(T)`, exactly like indexing an array.

## Structs

```marrow
struct Point {
    x: f64;
    y: f64;
}
```

- Fields are declared as `name: Type;` — note the **semicolon** after each field (not a comma).
- Fields are laid out in declaration order, each aligned to its own natural alignment, with the struct's total size rounded up to the alignment of its widest field (standard C-like layout — no `#[repr]`/packing controls exist yet).
- A struct can contain another struct by value, a pointer to itself (for linked structures — see `HashNode` in the [Map module](/standard-library/map/)), arrays, slices, or scalars. A struct **cannot** directly contain itself by value (infinite size) — the compiler rejects that with *"type récursif de taille infinie"* ("infinitely-sized recursive type").
- Struct values are always passed/returned through memory (never in registers) at the codegen level, using QBE's `l`-typed (pointer) calling convention plus an explicit `blit`/copy — this is invisible from Marrow source, but explains why passing a large struct by value copies it.

### Struct literals

```marrow
var p = Point { x: 1.0, y: 2.0 };
var p2 = Point { x: 1.0 };   // y defaults to 0.0 — fields are zero-initialized first
```

- `Name { field: expr, field2: expr2, ... }`, fields separated by **commas** here (unlike the `;` used in the declaration).
- Any field you don't mention is left at its zeroed value — the compiler zero-fills the whole struct before writing the fields you specified.
- Struct literals are disabled directly inside `if (...)`, `while (...)`, and the condition/post clauses of `for (...)` — this avoids ambiguity between `if Foo { ... }` (which would look like a struct literal) and the following block. Wrap the literal in parentheses if you need one there: `if (p == (Point { x: 0, y: 0 })) { ... }`.

## Type aliasing between built-ins and custom names

Any identifier that isn't one of the twelve built-in type names (`i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 bool rawptr`) is looked up as a struct name at the point of use. There is currently no `type Foo = Bar;` alias syntax exposed by the parser at the top level.