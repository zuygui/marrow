---
title: Control flow
description: if / else, while, for, break, continue and match.
sidebar:
  order: 5
---

Conditions are written without parentheses, bodies always use braces.

## `if` / `else`

```marrow
if x > 0 {
    println_i64(x);
} else if x == 0 {
    println("zero");
} else {
    println("negative");
}
```

### `if` as an expression

`if` can also produce a value. Both branches are required, and each one is a single expression with no `;`. Every branch must have the same type. Only the branch that is chosen runs.

```marrow
fn sign(x: i64) -> i64 {
    ret if x < 0 { -1 } else if x == 0 { 0 } else { 1 };
}

var label: *u8 = if n == 1 { "one" } else { "many" };
var total: i64 = (if big { 100 } else { 0 }) + extra;
```

Wrap it in parentheses when it starts a larger expression, as in the last line.

## `while`

```marrow
var i: i64 = 0;
while i < 10 {
    i += 1;
}
```

## `for`

`for init; cond; post { ... }` — each part is optional.

```marrow
for var i: i64 = 0; i < 10; i += 1 {
    println_i64(i);
}
```

## `break` and `continue`

`break;` leaves the innermost `while` or `for`; `continue;` jumps to the next iteration. In a `for` loop, `continue` still runs the `post` expression.

```marrow
for var i: i64 = 0; i < 100; i += 1 {
    if i % 2 == 0 { continue; }
    if i > 9 { break; }
    println_i64(i);        // 1 3 5 7 9
}
```

Using either outside a loop is error `E0416`. An infinite loop (`while true { ... }`) that contains a `break` is no longer treated as never terminating, so code after it is reachable.

## `match`

`match` works on [enums](/marrow/language/enums-and-match/) and is a statement. See that page for patterns, bindings and exhaustiveness. `break` and `continue` inside an arm act on the enclosing loop.
