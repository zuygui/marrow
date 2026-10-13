---
title: "Dir"
description: "Directories: listing, creating, walking."
sidebar:
  order: 24
---

Source: `std/dir.mw`.

Directories and file metadata (Linux today; other platforms report failure).


## Constants

| Name | Type | Value | Notes |
|---|---|---|---|
| `DIR_KIND_OTHER` | `i64` | `0` |  |
| `DIR_KIND_FILE` | `i64` | `1` |  |
| `DIR_KIND_DIR` | `i64` | `2` |  |
| `DIR_KIND_LINK` | `i64` | `3` |  |

## Types

### `DirEntry`

```marrow
struct DirEntry {
    name: *u8;
    kind: i64;
}
```

## Functions

### `dir_entry_free`

```marrow
fn dir_entry_free(e: *DirEntry)
```

### `dir_list_free`

```marrow
fn dir_list_free(v: *VecPtr)
```

Frees the list returned by dir\_list.


### `dir_list`

```marrow
fn dir_list(path: *u8) -> *VecPtr
```

Entries of a directory (without "." and ".."), sorted by name.  null on error.


### `file_stat`

```marrow
fn file_stat(path: *u8, mode: *i64, size: *i64, mtime: *i64) -> bool
```

Mode, size and modification time (Unix seconds) of a path.  false if it does not exist or stat is unavailable.


### `path_is_dir`

```marrow
fn path_is_dir(path: *u8) -> bool
```

### `path_is_file`

```marrow
fn path_is_file(path: *u8) -> bool
```

### `path_size`

```marrow
fn path_size(path: *u8) -> i64
```

Size in bytes, or -1.


### `path_mtime`

```marrow
fn path_mtime(path: *u8) -> i64
```

Modification time in Unix seconds, or -1.


### `file_rename`

```marrow
fn file_rename(from: *u8, to: *u8) -> bool
```

### `dir_remove`

```marrow
fn dir_remove(path: *u8) -> bool
```

Removes an empty directory.


### `dir_create_all`

```marrow
fn dir_create_all(path: *u8) -> bool
```

Creates a directory and any missing parents (like `mkdir -p`).


### `remove_all`

```marrow
fn remove_all(path: *u8) -> bool
```

Removes a file or a directory tree (symbolic links are removed, not followed).


### `cwd_get`

```marrow
fn cwd_get() -> *u8
```

Current working directory as a new string, or null.


### `cwd_set`

```marrow
fn cwd_set(path: *u8) -> bool
```

