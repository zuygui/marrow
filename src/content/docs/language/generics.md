---
title: Generics
description: Generic functions, structs and enums, monomorphized at compile time.
sidebar:
  order: 7
---

Functions, structs and enums can take type parameters in angle brackets. Marrow generates one ordinary copy per distinct list of type arguments (monomorphization), so there is no run-time cost and no boxing.

```marrow
fn max<T>(a: T, b: T) -> T {
    if a > b { ret a; }
    ret b;
}

struct Pair<A, B> { a: A; b: B; }
struct Node<T> { val: T; next: *Node<T>; }
```

## Using them

Type arguments are **always explicit**; there is no inference.

| Where | Syntax |
|---|---|
| In a type | `Pair<i64, u8>`, `*Node<i64>`, `Box<Box<i64>>` |
| Calling a generic function | `max::<i64>(3, 9)` |
| Struct literal | `Pair::<i64, u8> { a: 5, b: 6 }` |
| Enum variant | `Option::<i64>::Some(3)` |

Omitting them is error `E0422`, giving the wrong number is `E0422`, and using type arguments on something that is not generic is `E0423`.

```marrow
var p: Pair<i64, u8> = Pair::<i64, u8> { a: 5, b: 6 };
var best: i64 = max::<i64>(3, 9);
```

`sizeof(T)` works inside a generic body with `T` replaced by the real type:

```marrow
fn push<T>(head: *Node<T>, v: T) -> *Node<T> {
    var n: *Node<T> = cast(*Node<T>) alloc(sizeof(Node<T>));
    n.val = v;
    n.next = head;
    ret n;
}
```

## How it works

Each instance is named after the template and its arguments (`max::<i64>` becomes `max__i64`, `Pair<i64, u8>` becomes `Pair__i64__u8`). Diagnostics inside a generic body are reported when it is instantiated, against the generic's own source lines, so a generic that is never instantiated is never type-checked. The compiler stops after 4000 instances, which normally means a generic keeps instantiating itself with ever larger types (`E0430`).

Not supported yet: type inference, constraints/traits, generic methods, and local generic declarations.
