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

See [Modules & decorators](/language/decorators-and-modules/) for the full rules around `@extern` and `@export`.

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
- Inside a variadic function's body, the three built-in pseudo-functions `va_start()`, `va_arg(list, Type)` and `va_end(list)` drive the platform C variadic-argument ABI (they lower to QBE's `vastart`/`vaarg` instructions). See [Variadic functions](/language/variadic-functions/) for details and caveats.
- Calling a variadic function requires **at least** as many arguments as fixed parameters; any extra arguments are passed through with default C variadic promotion (small integers promoted to `i32`/`u32`, `f32` promoted to `f64`) — see [Types](/language/types/) for the promotion table.

## Return type inference from the binding

A function declared as `var name: T = fn(...) { ... }`-shaped local binding... actually doesn't exist in Marrow: functions are always declared with the `fn` keyword form shown above (`fn name(...) -> T { ... }` or `fn name(...) -> T => expr;`), never assigned like a value. Functions are **not first-class values** in this compiler — you cannot store a function in a variable, pass it as an argument, or return it from another function. The only thing you can do with a function name is call it directly (`f(args)`); referencing `f` on its own (e.g. `var g = f;`) is a compile error ("les fonctions ne sont pas des valeurs de première classe dans ce générateur").

## Calling functions

```marrow
var result = add(1, 2);
```

- Only direct calls to a named function are supported — there's no way to call through a function pointer or an arbitrary expression.
- Argument count must match exactly (unless the callee is variadic, in which case it must be at least the number of fixed parameters).
- Each argument is implicitly coerced to the corresponding parameter's declared type using the same rules as assignment (see [Types](/language/types/)).
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