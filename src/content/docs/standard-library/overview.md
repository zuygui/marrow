---
title: Standard Library overview
description: What ships in std/, how it's organized, and how to import it.
sidebar:
  order: 1
---

The standard library lives in the `std/` directory of the repository and is written **entirely in Marrow itself** (`.mw` files) — it's a thin, explicit wrapper around C standard library functions via `@extern`, plus a handful of hand-rolled data structures (dynamic arrays, hash maps, an arena allocator, a growable string type).

Every file in `std/` starts with `@no_main`, which is what marks them as libraries rather than standalone programs (see [Modules & decorators](/language/decorators-and-modules/)).

## Modules

| File | What it provides |
|---|---|
| [`io.mw`](/standard-library/io/) | Printing to stdout/stderr. |
| [`mem.mw`](/standard-library/mem/) | Raw allocation (`malloc`/`realloc`/`free`), `memcpy`/`memset` wrappers, and an arena allocator. |
| [`string.mw`](/standard-library/string/) | C-string helpers (`str_len`, `str_eq`, character classification) and a growable `String` type. |
| [`vec.mw`](/standard-library/vec/) | Growable arrays: `VecPtr` (of `rawptr`) and `VecI64` (of `i64`). |
| [`map.mw`](/standard-library/map/) | `LinearMap` (linear-scan key/value store) and `HashMap` (DJB2 hash map with chaining). |
| [`fs.mw`](/standard-library/fs/) | File I/O: open/close/read/write, whole-file read into a `String`. |
| [`sys.mw`](/standard-library/sys/) | Process exit, shelling out to `system(3)`, `panic`/`assert`. |
| `std.mw` | The umbrella module — `@import`s every file above, plus a `VERSION` constant. |

## Importing it

```marrow
@import("std")
```

This resolves to `std/std.mw` (see the exact resolution rules on the [Modules & decorators](/language/decorators-and-modules/) page) and transitively pulls in **every** module listed above. If you only need one module, you can import it directly instead, e.g.:

```marrow
@import("mem.mw")   // just the allocator, no io/string/vec/map/fs/sys
```

Modules `@import` each other where they depend on one another (e.g. `string.mw` imports `mem.mw` for `alloc`/`realloc`; `map.mw` imports `mem.mw`, `sys.mw`, `vec.mw` and `string.mw`) — the deduplication rule described on the modules page means importing the same file from multiple places is always safe.

## Design notes that apply across the whole standard library

- **Naming convention:** functions that are thin `@extern` bindings to a C library function are prefixed `c_` (`c_malloc`, `c_printf`, `c_fopen`, ...) and are *not* meant to be called directly from your code — call the Marrow-native wrapper next to them instead (`alloc`, `print`, `file_open_read`, ...).
- **No exceptions, no `Result`/`Option` types.** Errors are signaled the C way: a null pointer (`0`), a negative/zero return code, or (for genuinely unrecoverable situations) `panic()` which prints a message to stderr and calls `exit(1)`.
- **Manual memory management throughout.** Every `_new` function that allocates has a matching `_free` function; nothing here uses the arena allocator internally by default (it's opt-in — the arena is a tool *you* reach for, see [`mem.mw`](/standard-library/mem/)).
- Growable containers (`String`, `VecPtr`, `VecI64`) all follow the same doubling-growth strategy: when full, capacity is doubled and the buffer is `realloc`'d.