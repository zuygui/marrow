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

The implicit conversions are:

- an owner `^T` can be used where a borrow is expected (`peek(a)` lends `a`), and `&mut T` where `&T` is expected;
- as a **call argument**, an owner or a borrow can be given to a parameter of type `*T`: the owner is not moved, the callee just gets the address;
- a **fresh owner** (the result of a call) can be stored in a raw `*T` variable or field: nobody else holds it, so it is simply never freed.

Anything else needs an explicit `cast`. `cast(*T) owner` gives the memory up: the variable is empty afterwards and nothing frees the memory. A borrow can never be `null`; owners and raw pointers can.

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

## Unsafe

The compiler cannot check what goes through a raw pointer. These operations only compile inside an `unsafe { ... }` block or a `@trusted` or `@unsafe` function (error E0432):

- taking a raw address with `raw x`;
- dereferencing a raw pointer (`*p`), indexing it (`p[i]`) or doing arithmetic on it (`p + n`);
- reading a `^T` or `&T` field through a raw pointer to a struct, since that would go around the ownership checks;
- casting to `^T`, `&T` or `&mut T` from another kind of pointer.

Calling a function marked `@unsafe`, or an `@extern` function, also needs an unsafe context (error E0433): `dealloc`, `realloc`, `mem_copy`, `mem_zero`, the `raw_*` writers, `heap_free`, the `*_destroy` functions and the system call wrappers are of that kind.

Two decorators mark a function:

| Decorator | The body may use unsafe operations | Calling it needs `unsafe` |
| --- | --- | --- |
| `@trusted` | yes | no: it is safe to call, you vouch for it |
| `@unsafe` | yes | yes: the caller takes responsibility |

## Giving memory up

An owner stored in a raw pointer is never freed, so the conversion is spelled out: `cast(*T) owner` (error E0402 without it). The owner variable is empty afterwards, and using it is an error.

```marrow
holder.items = cast(*i64) make();     // on purpose: nothing frees it
```

## Structs that hold owners

A struct with a `^T` field (directly or in a nested struct) is moved like the `^T` itself: copying it, using it after a move, moving it in a loop or moving it out of a field are rejected (E0420, E0421, E0427). It is dropped at the end of its scope, which frees what it owns, and the value it replaces is dropped when it is overwritten.

```marrow
struct Pair { first: ^Counter; second: ^Counter; }

var p: Pair = make_pair();
var q: Pair = p;          // moved: `p` is empty
```

## Borrows that outlive their target

A borrow of a local variable cannot be returned, directly, through an owner that is lent, through a variable that holds the borrow, inside a struct or through a call that passes it on (E0426). It cannot be stored behind a pointer either (E0434). A struct that holds borrows keeps its targets borrowed while it lives.

## What is not checked yet

The checks are deliberately simple. Know their limits:

- Moving an owner or an owning struct out of a field is rejected (E0427), and overwriting a field does not drop the old value.
- A struct value that only has a `@drop` hook (no `^T` field) is not moved or dropped by value: use it behind a `^T`.
- Lifetimes are lexical (the block of the variable), and loans are only taken on local variables. A struct that holds a borrow and is built in several steps is not followed.
- Reading and writing plain data through a raw pointer to a struct (`p.count`) needs no `unsafe`; owner and borrow fields do.
- A program that never calls `dealloc` does not free anything: the drop calls it, nothing else does.
- Only the core containers of the standard library are owned (`VecPtr`, `VecI64`, `String`, `LinearMap`, `HashMap`). `Json`, `HttpMessage`, `Big`, `Tls`, `Cert` and the other types of `std/` still use plain pointers and `*_free` functions, and the compiler keeps raw pointers for its own data.
