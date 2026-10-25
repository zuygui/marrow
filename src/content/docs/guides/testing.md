---
title: Testing
description: "@test functions, std/test.mw and marrow test."
sidebar:
  order: 3
---

Marrow has tests built in: no framework to install, no configuration.

`marrow test [filter]` runs:

- every `@test fn name() { ... }` found in `src/` and `tests/`: each one runs in its own process, and a failed check ends it with status `1`;
- every program with a `main` in `tests/`: it passes when it exits with status `0`.

`[dev-dependencies]` are available while testing. The filter keeps the tests whose name contains it. The output ends with `test result: ok. N passed; M failed`, and the exit code is `1` when something failed. See [`@test`](/marrow/language/decorators-and-modules/#test) and `std/test.mw`.

```marrow
@import("std/std.mw")
@import("std/test.mw")

@test fn adds() {
    check_eq_i64(1 + 1, 2, "one plus one");
}
```


## Helpers

`std/test.mw` ([reference](/marrow/standard-library/test/)) has the checks. A failed check prints what went wrong on stderr and ends the process with status `1`. Because every test runs in its own process, one failing test never hides another.

## Where tests live

- **In `src/`**, next to the code, as `@test` functions.
- **In `tests/`**, as files with `@test` functions or as programs with a `main`.

```
app/
├── marrow.toml
├── src/main.mw
└── tests/
    ├── parser.mw        (@test functions)
    └── end_to_end.mw    (a program with main: exit 0 means it passed)
```

Run one test or a group with a filter: `marrow test parse`.
