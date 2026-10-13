---
title: Enums and match
description: C-like enums, enums with payloads, generic enums and exhaustive match statements.
sidebar:
  order: 6
---

## Declaring an enum

Variants end with `;`. A variant may carry values, written like a parameter list of types.

```marrow
enum Color { Red; Green; Blue; }

enum Shape {
    Circle(i64);
    Rect(i64, i64);
    Empty;
}
```

## C-like enums

An enum whose variants carry nothing is an `i64` at run time. `Color::Red` is the constant `0`, `Color::Green` is `1`, and so on, so values can be stored, compared with `==` and passed around like integers.

```marrow
var c: Color = Color::Green;
if c == Color::Green { println("green"); }
```

Because the type is just an integer, the compiler does not stop you from passing a plain `i64` where a `Color` is expected.

## Enums with payloads

Variants with values are built by calling them:

```marrow
var s: Shape = Shape::Rect(3, 4);
var e: Shape = Shape::Empty;      // a variant without values needs no call
```

Under the hood the enum is a struct `{ tag: i64; data: [W]u64 }` where `W` is large enough for the biggest variant, and every variant has a constructor function `Shape__Rect`. `sizeof(Shape)` is therefore `8 + 8 * W` bytes. An enum cannot contain itself by value (error `E0430`); use a pointer.

## `match`

```marrow
fn area(s: Shape) -> i64 {
    match s {
        Shape::Circle(r)  => { ret r * r * 3; }
        Shape::Rect(w, h) => { ret w * h; }
        Shape::Empty      => { ret 0; }
    }
}
```

- Each arm is `Enum::Variant(bindings) => { block }`. Bindings are **copies** of the payload values; writing to them does not change the enum. Use `_` to ignore a value.
- The scrutinee is evaluated once.
- The match must be **exhaustive**, or end with an `else` arm (`E0428` otherwise). `else` must be last (`E0427`).
- All patterns must belong to the same enum (`E0426`), and the number of bindings must match the variant (`E0429`).
- Matching the same variant twice is warning `W0006`.
- `match` is a statement, not an expression: use `ret` or assign to a variable inside the arms.

```marrow
match t {
    Tok::Num(n) => { total += n; }
    Tok::Plus   => { continue; }
    else        => { break; }
}
```

## Generic enums

Enums can take type parameters. Type arguments are always explicit: in expressions and patterns they follow `::`.

```marrow
enum Option<T> { Some(T); None; }

fn find(v: i64) -> Option<i64> {
    if v > 0 { ret Option::<i64>::Some(v); }
    ret Option::<i64>::None;
}

match find(41) {
    Option::<i64>::Some(x) => { println_i64(x); }
    Option::<i64>::None    => { println("none"); }
}
```

See [Generics](/marrow/language/generics/).
