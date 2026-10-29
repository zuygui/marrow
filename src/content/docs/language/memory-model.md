---
title: Memory model
description: Pointer kinds, ownership and moves, borrows, automatic drop and the limits of the checks.
sidebar:
  order: 8
---

Marrow has three kinds of pointers. The compiler checks the first two; the third is the escape hatch you already know from C.

| Type | Kind | Meaning |
| --- | --- | --- |
| `^T` | owning | The one pointer responsible for the memory. When it goes out of scope, the memory is freed. |
| `&T`, `&mut T` | borrow | A loan of someone else's memory: read-only, or read and write. Never null. |
| `*T` | raw | A plain address. Nothing is checked. |

```marrow
fn make(v: i64) -> ^i64 {
    var p: ^i64 = cast(^i64) alloc(8);
    *p = v;
    ret p;
}

fn peek(p: &i64) -> i64 { ret *p; }
fn bump(p: &mut i64) { *p += 1; }
```

## Taking addresses

| Expression | Result |
| --- | --- |
| `&x` | a `&T` borrow of `x` |
| `&mut x` | a `&mut T` borrow of `x` |
| `raw x` | a raw `*T` address of `x` |

Only two implicit conversions exist: an owner `^T` can be used where a borrow is expected (`peek(a)` lends `a`), and `&mut T` can be used where `&T` is expected. Anything else needs an explicit `cast`. A borrow can never be `null`; owners and raw pointers can.

## Ownership and moves

Assigning an owner, passing it to a function or returning it **moves** it: the source variable is empty until it is assigned again, and using it is an error.

```marrow
var a: ^i64 = make(10);
var b: ^i64 = a;        // moved: `a` is empty now
println_i64(peek(b));
a = make(7);            // filled again, `a` can be used
```

```text
error[E0420]: use of moved value `a`
error[E0421]: value `a` is moved in a loop
error[E0427]: cannot move an owner out of a field, an element or a pointer
```

The checker follows `if`/`else`, `match` and loops: a value moved in one branch is considered moved afterwards, and a value moved inside a loop body must be filled again before the next iteration.

## Borrows

A borrow lasts as long as the variable that holds it, within its block. While a borrow of `x` exists:

- `&mut x` cannot be taken again, nor `&x` (E0423, E0424);
- `x` cannot be assigned (E0424) or moved (E0425);
- a function cannot receive the same variable twice if one of the two borrows is mutable;
- nothing can be written through a `&T` (E0422);
- a function cannot return a borrow of one of its own locals (E0426).

## Automatic drop

An owner is freed at the end of its scope, on `ret`, `break` and `continue`, unless it was moved. Assigning to an owner variable frees the value it held. Freeing calls `dealloc`, so only memory that came from `alloc` should be put in a `^T`.

A struct can run code before its memory is freed, to release what it holds:

```marrow
@drop(buffer_release)
struct Buffer { data: *u8; size: i64; }

fn buffer_release(b: &mut Buffer) { dealloc(b.data); }
```

The hook runs when a `^Buffer` is freed, then the struct itself is freed. Owners stored in the fields of the struct are dropped too.

## Constants

`const` declares a binding that cannot be assigned or borrowed as mutable (E0428). It works for globals and locals, and needs a type and a value.

```marrow
const LIMIT: i64 = 10;
```

## Unsafe blocks

`unsafe { ... }` is accepted and marks a block where you take responsibility for what the compiler cannot check, such as casting a raw pointer to an owner. It has no effect yet: the checks are the same inside and outside the block.

## What is not checked yet

The checks are deliberately simple. Know their limits:

- Struct **values** that contain owners are not tracked: they are not moved or dropped by value. Only `^T` variables are.
- Moving an owner out of a field is rejected (E0427), and overwriting a field does not drop the old value.
- Lifetimes are lexical (the block of the variable), and loans are only taken on local variables.
- Nothing stops you from writing through a raw `*T` that points into a `&T`.
- A program that never calls `dealloc` does not free anything: the drop calls it, nothing else does.
- The standard library and the compiler still use raw pointers; they are not migrated to `^T` yet.
