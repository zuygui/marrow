---
title: "Map"
description: Hash maps and linear maps.
sidebar:
  order: 19
---

Source: `std/map.mw`.

`LinearMap` scans linearly and is meant for tiny maps; `HashMap` is a chained hash map.

## Types

### `LinearMapEntry`

```marrow
struct LinearMapEntry {
    key: *String;
    value: rawptr;
}
```

### `LinearMap`

```marrow
struct LinearMap {
    entries: *VecPtr; // Stocke des *LinearMapEntry
}
```

### `HashNode`

```marrow
struct HashNode {
    key: *String;
    value: rawptr;
    hash: i64;
    next: *HashNode;
}
```

### `HashMap`

```marrow
struct HashMap {
    buckets: *VecPtr;
    capacity: i64;
    count: i64;
}
```

## Functions

### `linear_map_new`

```marrow
fn linear_map_new(initial_cap: i64) -> *LinearMap
```

### `linear_map_put`

```marrow
fn linear_map_put(map: *LinearMap, key: *String, value: rawptr)
```

### `linear_map_get`

```marrow
fn linear_map_get(map: *LinearMap, key: *String) -> rawptr
```

### `linear_map_free`

```marrow
fn linear_map_free(map: *LinearMap)
```

### `hash_djb2`

```marrow
fn hash_djb2(s: *String) -> i64
```

### `hash_map_new`

```marrow
fn hash_map_new(capacity: i64) -> *HashMap
```

### `hash_map_grow`

```marrow
fn hash_map_grow(map: *HashMap)
```

Doubles the number of buckets and relinks every node.


### `hash_map_put`

```marrow
fn hash_map_put(map: *HashMap, key: *String, value: rawptr)
```

### `hash_map_get`

```marrow
fn hash_map_get(map: *HashMap, key: *String) -> rawptr
```

### `hash_map_contains`

```marrow
fn hash_map_contains(map: *HashMap, key: *String) -> bool
```

### `hash_map_len`

```marrow
fn hash_map_len(map: *HashMap) -> i64
```

### `hash_map_remove`

```marrow
fn hash_map_remove(map: *HashMap, key: *String) -> rawptr
```

Removes `key` and returns its value (0 when absent). The key string itself is not freed.


### `hash_map_keys`

```marrow
fn hash_map_keys(map: *HashMap) -> *VecPtr
```

Keys as a vector of `*String` (the keys are shared, not copied).


### `hash_map_free`

```marrow
fn hash_map_free(map: *HashMap)
```

Frees the map and its nodes; keys and values are left to the caller.


