---
title: "Vec"
description: Growable arrays.
sidebar:
  order: 18
---

Source: `std/vec.mw`.

The library ships two concrete arrays: `VecPtr` for pointers and `VecI64` for integers. The language now has [generics](/marrow/language/generics/), but the std has not been migrated to them yet.

## Types

### `VecPtr`

```marrow
struct VecPtr {
    data: rawptr;
    len: i64;
    cap: i64;
}
```

### `VecI64`

```marrow
struct VecI64 {
    data: rawptr;
    len: i64;
    cap: i64;
}
```

## Functions

### `vec_ptr_new`

```marrow
fn vec_ptr_new(initial_cap: i64) -> ^VecPtr
```

### `vec_ptr_push`

```marrow
fn vec_ptr_push(v: *VecPtr, item: rawptr)
```

### `vec_ptr_get`

```marrow
fn vec_ptr_get(v: *VecPtr, index: i64) -> rawptr
```

### `vec_ptr_free`

```marrow
fn vec_ptr_free(v: ^VecPtr)
fn vec_ptr_destroy(v: *VecPtr)
```

A `^VecPtr` frees itself (the buffer and the struct) when it goes out of scope. `vec_ptr_free` takes the owner and drops it at once. `vec_ptr_destroy` frees a vector that is only known by a plain pointer, such as a struct field: you were its owner, and it must not be used afterwards.

### `vec_ptr_set`

```marrow
fn vec_ptr_set(vec: *VecPtr, index: i64, value: rawptr)
```

### `vec_i64_new`

```marrow
fn vec_i64_new(initial_cap: i64) -> ^VecI64
```

### `vec_i64_push`

```marrow
fn vec_i64_push(v: *VecI64, val: i64)
```

### `vec_i64_get`

```marrow
fn vec_i64_get(v: *VecI64, index: i64) -> i64
```

### `vec_i64_free`

```marrow
fn vec_i64_free(v: ^VecI64)
fn vec_i64_destroy(v: *VecI64)
```

Same rules as `vec_ptr_free` and `vec_ptr_destroy`.

### `vec_i64_set`

```marrow
fn vec_i64_set(v: *VecI64, index: i64, val: i64)
```

### `vec_ptr_len`

```marrow
fn vec_ptr_len(v: *VecPtr) -> i64
```

### `vec_i64_len`

```marrow
fn vec_i64_len(v: *VecI64) -> i64
```

### `vec_ptr_pop`

```marrow
fn vec_ptr_pop(v: *VecPtr) -> rawptr
```

Removes and returns the last element (0 when empty).


### `vec_i64_pop`

```marrow
fn vec_i64_pop(v: *VecI64) -> i64
```

### `vec_i64_sort`

```marrow
fn vec_i64_sort(v: *VecI64)
```

In-place ascending sort (heap sort: no recursion, no extra memory).


### `vec_i64_sift`

```marrow
fn vec_i64_sift(v: *VecI64, root: i64, n: i64)
```

