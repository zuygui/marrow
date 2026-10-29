---
title: "Option and Result"
description: Optional values and results, as an opt-in module.
sidebar:
  order: 40
---

Source: `std/option.mw`. Not part of `std/std.mw`: names are shared by the whole program, so a program that defines its own `Option` or `Result` keeps working. Import it where you need it.

```marrow
@import("std/option.mw")

enum Option<T> { Some(T); None; }
enum Result<T, E> { Ok(T); Err(E); }
```

Generic functions take their types explicitly, and patterns write them too.

| Function | Returns |
| --- | --- |
| `option_is_some::<T>(o)` / `option_is_none::<T>(o)` | `bool` |
| `option_unwrap_or::<T>(o, fallback)` | the value, or `fallback` |
| `result_is_ok::<T, E>(r)` / `result_is_err::<T, E>(r)` | `bool` |
| `result_unwrap_or::<T, E>(r, fallback)` | the value, or `fallback` |
| `result_err_or::<T, E>(r, fallback)` | the error, or `fallback` |
| `result_ok::<T, E>(r)` | `Option<T>`, dropping the error |

```marrow
match parse_digit('7') {
    Result::<i64, i64>::Ok(v) => { println_i64(v); }
    Result::<i64, i64>::Err(e) => { println_i64(e); }
}
```
