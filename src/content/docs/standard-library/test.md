---
title: "Test"
description: Checks for functions marked @test, run with marrow test.
sidebar:
  order: 60
---

Source: `std/test.mw`.

Helpers for functions marked [`@test`](/marrow/language/decorators-and-modules/#test). A failing check prints what went wrong on stderr and ends the process with status `1`: every test runs in its own process (see [Testing](/marrow/guides/testing/)).

```marrow
@import("std/std.mw")
@import("std/test.mw")

@test fn arithmetic() {
    check(1 + 1 == 2, "one plus one");
    check_eq_i64(6 * 7, 42, "the answer");
    check_eq_str("mar", "mar", "same text");
}
```

## Functions

| Function | Description |
|---|---|
| `test_fail(message: *u8)` | Prints `assertion failed: <message>` and exits with status `1`. |
| `check(condition: bool, message: *u8)` | Fails with `message` unless `condition` holds. |
| `check_eq_i64(actual: i64, expected: i64, message: *u8)` | Fails unless the two integers are equal, and prints both values. |
| `check_eq_str(actual: *u8, expected: *u8, message: *u8)` | Fails unless the two strings are equal, and prints both. |
