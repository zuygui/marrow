---
title: Functions
description: Declaring functions, expression-bodied functions, variadics, and how they're linked.
sidebar:
  order: 4
---

## Basic syntax

```marrow
fn add(a: i64, b: i64) -> i64 {
    ret a + b;
}
```

- Parameters are `name: Type`, comma-separated.
- The return type is introduced with `->`. If omitted, the function returns nothing (`void`); a bare `ret;` (or falling off the end of the body) is fine in that case.
- The function body is a `{ ... }` block, using `ret expr;` (or bare `ret;`) to return.
- If the body doesn't explicitly `ret` on every path, the compiler inserts an implicit `ret` at the end (returning a zeroed value of the declared return type, or nothing for `void`) — there is no "missing return" error.

## Expression-bodied functions

```marrow
fn square(x: i64) -> i64 => x * x;
```

`=> expr;` is sugar for a body containing a single `ret expr;`. It's purely a parsing convenience — the generated code is identical to the block form.

## No-body declarations (`@extern`)

A function can be declared **without** a body, ending directly in `;` after the signature. This is only legal when the function is decorated `@extern` — it tells the compiler "this function is implemented elsewhere; just generate a call to it":

```marrow
@extern("malloc") fn c_malloc(size: i64) -> *u8;
```

See [Modules & decorators](/marrow/language/decorators-and-modules/) for the full rules around `@extern` and `@export`.

- A function **without** `@extern` **must** have a body — the compiler rejects a bodiless, non-extern declaration.
- A function **with** `@extern` **must not** have a body — the compiler rejects that combination too, since it would be ambiguous (foreign symbol *or* local definition?).

## Variadic parameters

```marrow
fn logf(fmt: *u8, ...) {
    var args = va_start();
    var n: i64 = va_arg(args, i64);
    va_end(args);
}
```

- `...` as the last item in the parameter list marks a function as variadic (C-style). It's purely a marker in the signature — variadic parameters have no name and no fixed type in Marrow itself.
- Inside a variadic function's body, the three built-in pseudo-functions `va_start()`, `va_arg(list, Type)` and `va_end(list)` drive the platform C variadic-argument ABI (they lower to QBE's `vastart`/`vaarg` instructions). See [Variadic functions](/marrow/language/variadic-functions/) for details and caveats.
- Calling a variadic function requires **at least** as many arguments as fixed parameters; any extra arguments are passed through with default C variadic promotion (small integers promoted to `i32`/`u32`, `f32` promoted to `f64`) — see [Types](/marrow/language/types/) for the promotion table.

## Function pointers

Functions are values. Their type is written `fn(A, B) -> R` (omit `-> R` for a function that returns nothing), and the name of a function can be used wherever a value of that type is expected:

```marrow
fn double(x: i64) -> i64 { ret x * 2; }
fn square(x: i64) -> i64 { ret x * x; }

fn apply(f: fn(i64) -> i64, v: i64) -> i64 {
    ret f(v);
}

fn pick(which: i64) -> fn(i64) -> i64 {
    if which == 0 { ret double; }
    ret square;
}

var f: fn(i64) -> i64 = double;
f = square;
apply(double, 21);   // 42
pick(1)(10);         // 100
```

- A function pointer can be stored in a variable, a struct field or an array element, passed as an argument and returned from a function. Calling through a field works as you'd expect (`handler.run(8)`).
- Calls are type-checked against the pointer's signature: wrong argument count or types are ordinary errors.
- Function pointers can be compared with `==` and `!=`, including against `null`, and cast to and from `rawptr` (`cast(rawptr) f`, `cast(fn(i64) -> i64) raw`) to cross an untyped boundary.
- There are no closures: a function value carries no captured environment, so pass state through an explicit argument, typically a `rawptr`. This is how [tasks](/marrow/guides/async/) work: `task_spawn(f: fn(rawptr) -> i64, arg: rawptr)`.

## Calling functions

```marrow
var result = add(1, 2);
```

- You can call a named function directly or call through any expression of [function pointer](#function-pointers) type.
- Argument count must match exactly (unless the callee is variadic, in which case it must be at least the number of fixed parameters).
- Each argument is implicitly coerced to the corresponding parameter's declared type using the same rules as assignment (see [Types](/marrow/language/types/)).
- Struct/array/slice arguments are passed by copying the whole value into a fresh stack slot at the call site (pass-by-value semantics — the callee never mutates the caller's copy through a plain value parameter; pass a pointer explicitly if you need that).

## Recursion

Plain recursion works exactly as you'd expect — a function can call itself, direct or mutual, since all top-level function signatures are registered before any function body is generated.

```marrow
fn fib(n: i64) -> i64 {
    if (n < 2) {
        ret n;
    }
    ret fib(n - 1) + fib(n - 2);
}
```