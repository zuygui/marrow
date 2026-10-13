---
title: "Path"
description: Path joining and splitting.
sidebar:
  order: 25
---

Source: `std/path.mw`.

Path helpers for '/'-separated paths.  Functions returning \*u8 allocate a new string (release it with dealloc).


## Functions

### `path_is_absolute`

```marrow
fn path_is_absolute(p: *u8) -> bool
```

### `path_join`

```marrow
fn path_join(a: *u8, b: *u8) -> *u8
```

Joins two paths with exactly one separator; an absolute `b` wins.


### `path_basename`

```marrow
fn path_basename(p: *u8) -> *u8
```

Last component ("/a/b/" -> "b", "/" -> "/").


### `path_dirname`

```marrow
fn path_dirname(p: *u8) -> *u8
```

Everything but the last component ("/a/b" -> "/a", "b" -> ".", "/a" -> "/").


### `path_extension`

```marrow
fn path_extension(p: *u8) -> *u8
```

File extension including the dot, as a pointer into `p` ("a/b.tar.gz" -> ".gz"), or "" when the last component has none (a leading dot does not count).


### `path_normalize`

```marrow
fn path_normalize(p: *u8) -> *u8
```

Resolves ".", ".." and repeated separators lexically.


