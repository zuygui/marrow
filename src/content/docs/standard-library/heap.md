---
title: "Heap"
description: The libc-free allocator behind `alloc`.
sidebar:
  order: 13
---

Source: `std/heap.mw`.

The allocator is size-class based and gets its pages from `sys_mmap_anon`. Nothing in it calls libc.

A small general-purpose allocator on top of raw pages (std/os.mw).

- Requests up to 64 KiB are rounded to a power-of-two size class

(16 B .. 64 KiB). Freed blocks go on a per-class free list.

- Larger requests get their own mapping, returned to the OS on free.
- Every block has a 16-byte header: word 0 = class index (or total

mapping size for large blocks), word 1 = -1 for large blocks, else 0.


## Constants

| Name | Type | Value | Notes |
|---|---|---|---|
| `HEAP_TABLE` | `i64` | `0` |  |
| `HEAP_CUR` | `i64` | `0` |  |
| `HEAP_END` | `i64` | `0` |  |

## Functions

### `heap_word`

```marrow
fn heap_word(addr: i64) -> *i64
```

### `heap_class`

```marrow
fn heap_class(size: i64) -> i64
```

### `heap_class_size`

```marrow
fn heap_class_size(c: i64) -> i64
```

Size in bytes of a size class: 16 \* 2^c (the language has no shift operator).


### `heap_init`

```marrow
fn heap_init() -> i64
```

### `heap_alloc`

```marrow
fn heap_alloc(size: i64) -> *u8
```

### `heap_capacity`

```marrow
fn heap_capacity(ptr: *u8) -> i64
```

### `heap_free`

```marrow
fn heap_free(ptr: *u8)
```

### `heap_realloc`

```marrow
fn heap_realloc(ptr: *u8, new_size: i64) -> *u8
```

### `heap_alloc_zeroed`

```marrow
fn heap_alloc_zeroed(size: i64) -> *u8
```

