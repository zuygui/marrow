---
title: Variables
description: The var keyword, type inference, scoping, and the difference between global and local variables.
sidebar:
  order: 3
---

Marrow has a single declaration keyword for variables: `var`. There is no separate `const`/`let` distinction at the syntax level — mutability and where a variable lives (global vs. local, stack slot vs. static data) are inferred from *where* the declaration appears.

## Syntax

```marrow
var name = expr;
var name: Type = expr;
```

The type annotation is optional. When omitted, the type is inferred from the initializer expression's own type (an integer literal defaults to `i64`, a float literal to `f64`, a string literal to `*u8`, a char literal to `u8`, `null` to `rawptr`, etc. — see the coercion rules on the [Types](/language/types/) page).

## Local variables

Inside a function body, `var` declares a stack-allocated local:

```marrow
fn distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    var dx: f64 = x2 - x1;
    var dy = y2 - y1;          // type inferred as f64 from the expression
    ret dx * dx + dy * dy;
}
```

- The initializer can be **any expression** — function calls, arithmetic, struct literals, casts, everything. There's no restriction to compile-time constants for locals.
- Locals are block-scoped: a `{ ... }` block, the body of an `if`/`while`/`for`, all open a new scope, and variables declared inside are not visible after the block ends.
- Re-declaring the same name in a nested block shadows the outer one for the rest of that block.
- All locals are mutable — reassign with plain `=` (or `+= -= *= /=`) on any lvalue: `x = x + 1;`.

## Global variables

At the top level (outside any function), `var` declares a **global**:

```marrow
var VERSION = 0.1;
@export var PI: f64 = 3.14159;
```

Globals are compiled into static data. Because of that, **the initializer must be a compile-time constant expression** — the compiler only knows how to fold:

- integer, float, bool, string, char and `null` literals,
- unary negation (`-x`) of a numeric literal,
- binary `+ - * /` combining two literals of the same kind (int-with-int or float-with-float).

Anything else (a function call, a reference to another variable, a struct literal, mixed int/float arithmetic) is rejected with *"expression non constante"* ("non-constant expression"). If you need computed initialization, do it inside `main` (or another function) into a variable instead.

- Global declarations can be decorated with `@export` to make the resulting symbol visible for linking against from other object files (see [Modules & decorators](/language/decorators-and-modules/)). `@extern` cannot be used on a `var` — only on functions.
- A global's type, if omitted, is inferred purely from the *shape* of the literal expression (see the list above) — not from arbitrary constant folding results.

## Assignment operators

Available on any scalar lvalue: `= += -= *= /=`. Compound assignment operators are **not supported on struct/array/slice-typed lvalues** — assign field-by-field instead.

```marrow
var count: i64 = 0;
count += 1;
count *= 2;
```