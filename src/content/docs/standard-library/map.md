---
title: "map — LinearMap and HashMap"
description: Key/value stores from std/map.mw — a simple linear-scan map and a DJB2 hash map.
sidebar:
  order: 6
---

Source: `std/map.mw`. Imports [`mem.mw`](/marrow/standard-library/mem/), [`sys.mw`](/marrow/standard-library/sys/), [`vec.mw`](/marrow/standard-library/vec/) and [`string.mw`](/marrow/standard-library/string/).

Both map types use `*String` (see the [string module](/marrow/standard-library/string/)) as their key type — not raw `*u8` C-strings — and store an arbitrary `rawptr` as the value, which you cast to/from your real pointer type at the call site.

## `LinearMap`

A minimal map backed by a plain `VecPtr` of entries, searched **linearly** on every lookup. Simple and fine for small maps; `O(n)` per `get`/`put`.

```marrow
struct LinearMapEntry {
    key: *String;
    value: rawptr;
}

struct LinearMap {
    entries: *VecPtr;
}
```

### `linear_map_new(initial_cap: i64) -> *LinearMap`
Creates a new, empty map with room for `initial_cap` entries before its backing vector needs to grow.

### `linear_map_put(map: *LinearMap, key: *String, value: rawptr)`
Appends a new `(key, value)` entry. **Does not check for an existing key first** — putting the same key twice adds a second entry rather than overwriting the first. Because lookups scan from the most-recently-added entry backwards (see below), a later `put` for the same key effectively shadows the earlier one for `linear_map_get`, but both entries still exist in memory (and both get freed by `linear_map_free`).

### `linear_map_get(map: *LinearMap, key: *String) -> rawptr`
Scans entries from **last-added to first-added** and returns the value of the first key that compares equal (via `string_eq`). Returns a null pointer if the map or key is null, or if no entry matches.

### `linear_map_free(map: *LinearMap)`
Frees every entry struct, then the backing vector, then the map itself.

## `HashMap`

A DJB2-hashed map with separate chaining (a linked list per bucket) — `O(1)` average-case lookup/insert.

```marrow
struct HashNode {
    key: *String;
    value: rawptr;
    hash: i64;
    next: *HashNode;
}

struct HashMap {
    buckets: *VecPtr;   // one *HashNode chain head per bucket
    capacity: i64;      // fixed bucket count
    count: i64;         // number of entries currently stored
}
```

### `hash_djb2(s: *String) -> i64`
The hash function used internally: the classic DJB2 algorithm (`hash = hash * 33 + byte`, seeded at `5381`), with the result forced non-negative (negated if it came out negative). You generally won't need to call this directly.

### `hash_map_new(capacity: i64) -> *HashMap`
Creates a new map with a **fixed** number of buckets (`capacity`) — the bucket count does **not** grow automatically as entries are added, so pick a capacity with your expected entry count in mind (more entries than buckets just means longer chains, not incorrect behavior).

### `hash_map_put(map: *HashMap, key: *String, value: rawptr)`
Inserts or updates: if `key` is already present in its bucket's chain, its value is overwritten in place; otherwise a new `HashNode` is allocated and pushed onto the front of the bucket's chain, and `map.count` is incremented.

### `hash_map_get(map: *HashMap, key: *String) -> rawptr`
Walks the target bucket's chain looking for a matching key (via `string_eq`) and returns its value, or a null pointer if not found.

:::caution[No `hash_map_free`]
Unlike every other constructor in the standard library, `map.mw` does **not** currently provide a `hash_map_free` function — there's no built-in way to release a `HashMap`'s nodes, buckets vector and struct in one call. If you need to tear one down, you'll have to free the `HashNode` chains, the buckets `VecPtr`, and the `HashMap` struct manually (mirroring what `linear_map_free` does for `LinearMap`), or route the whole map through an [`Arena`](/marrow/standard-library/mem/) and reset/free the arena instead of freeing individual pieces.
:::

## Example

```marrow
@import("map.mw")

fn build() {
    var m: *HashMap = hash_map_new(16);

    var k1: *String = string_from("name");
    hash_map_put(m, k1, string_from("marrow"));

    var v: *String = cast(*String) hash_map_get(m, k1);
    if (v != 0) {
        println(v.data);
    }
}
```
