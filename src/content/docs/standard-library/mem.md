---
title: "Memory"
description: Allocation, copying and arenas.
sidebar:
  order: 12
---

Source: `std/mem.mw`.

Thin, stable names over the allocator in [`heap.mw`](/marrow/standard-library/heap/) and the primitives in [`raw.mw`](/marrow/standard-library/raw/). Every allocating function has a matching free.

Thin, stable names over the libc-free allocator (std/heap.mw) and the memory primitives (std/raw.mw).


## Types

### `Arena`

```marrow
struct Arena {
    buffer: *u8;
    capacity: i64;
    offset: i64;
}
```

## Functions

### `alloc`

```marrow
fn alloc(size: i64) -> *u8
```

### `realloc`

```marrow
fn realloc(ptr: *u8, new_size: i64) -> *u8
```

### `dealloc`

```marrow
fn dealloc(ptr: *u8)
```

### `mem_copy`

```marrow
fn mem_copy(dest: *u8, src: *u8, count: i64)
```

### `mem_zero`

```marrow
fn mem_zero(dest: *u8, count: i64)
```

### `arena_new`

```marrow
fn arena_new(capacity: i64) -> *Arena
```

### `arena_alloc`

```marrow
fn arena_alloc(a: *Arena, size: i64) -> *u8
```

### `arena_reset`

```marrow
fn arena_reset(a: *Arena)
```

### `arena_free`

```marrow
fn arena_free(a: *Arena)
```

