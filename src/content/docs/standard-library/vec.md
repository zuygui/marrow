---
title: "vec — growable arrays"
description: VecPtr and VecI64, the growable array types from std/vec.mw.
sidebar:
  order: 5
---

Source: `std/vec.mw`. Imports [`mem.mw`](/marrow/standard-library/mem/).

There is no generic `Vec<T>` — Marrow has no generics — so the standard library ships two concrete, hand-written growable arrays: `VecPtr` (elements are `rawptr`) and `VecI64` (elements are `i64`). Both share the exact same shape and growth strategy; pick whichever matches what you're storing (store an `i64`-encoded value or index into `VecI64`, store heap pointers — including pointers to your own structs, cast through `rawptr` — into `VecPtr`).

Both are always used through a pointer (`*VecPtr` / `*VecI64`), returned by their `_new` constructor.

## `VecPtr` — a vector of pointers

```marrow
struct VecPtr {
    data: rawptr;
    len: i64;
    cap: i64;
}
```

### `vec_ptr_new(initial_cap: i64) -> *VecPtr`
Allocates a new, empty vector. `initial_cap` is the starting element capacity; values below `4` are bumped up to `4`.

### `vec_ptr_push(v: *VecPtr, item: rawptr)`
Appends a pointer to the end, doubling capacity (and `realloc`ing the backing buffer) whenever the vector is full.

### `vec_ptr_get(v: *VecPtr, index: i64) -> rawptr`
Returns the element at `index`, or a null pointer (`0`) if `index` is out of bounds (negative or `>= len`) — bounds are checked, not asserted/panicked.

### `vec_ptr_set(vec: *VecPtr, index: i64, value: rawptr)`
Overwrites the element at `index` in place. Silently does nothing if `vec` is null or `index` is out of bounds — it does **not** grow the vector or append; use `vec_ptr_push` to add new elements.

### `vec_ptr_free(v: *VecPtr)`
Frees the backing data buffer, then the `VecPtr` struct itself.

## `VecI64` — a vector of 64-bit integers

```marrow
struct VecI64 {
    data: rawptr;
    len: i64;
    cap: i64;
}
```

Same API shape as `VecPtr`, storing `i64` values instead of pointers:

- `vec_i64_new(initial_cap: i64) -> *VecI64`
- `vec_i64_push(v: *VecI64, val: i64)`
- `vec_i64_get(v: *VecI64, index: i64) -> i64` — returns `0` (not a distinguishable "not found") when out of bounds.
- `vec_i64_free(v: *VecI64)`

## Example

```marrow
@import("vec.mw")

fn sum(numbers: *VecI64) -> i64 {
    var total: i64 = 0;
    var i: i64 = 0;
    while (i < numbers.len) {
        total = total + vec_i64_get(numbers, i);
        i = i + 1;
    }
    ret total;
}

fn build() -> i64 {
    var v: *VecI64 = vec_i64_new(4);
    vec_i64_push(v, 10);
    vec_i64_push(v, 20);
    vec_i64_push(v, 30);

    var s: i64 = sum(v);
    vec_i64_free(v);
    ret s;
}
```

:::note
Neither `vec_ptr_get`/`vec_i64_get` returning `0` nor `vec_ptr_set`/out-of-bounds writes being silently ignored are reported as errors — always check `index` against `.len` yourself if `0` is a value you'd otherwise store legitimately.
:::
