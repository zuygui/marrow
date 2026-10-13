---
title: "FS"
description: Reading and writing files.
sidebar:
  order: 23
---

Source: `std/fs.mw`.

File helpers built on `sys_open`, `sys_read` and `sys_write`. Errors are reported as a negative value or a null pointer, never as an exception.

Files on top of raw system calls. A file handle is a `rawptr` holding the file descriptor plus one, so that `null` can mean "could not open".


## Functions

### `file_handle`

```marrow
fn file_handle(fd: i64) -> rawptr
```

### `file_fd`

```marrow
fn file_fd(handle: rawptr) -> i32
```

### `file_open_read`

```marrow
fn file_open_read(path: *u8) -> rawptr
```

### `file_open_write`

```marrow
fn file_open_write(path: *u8) -> rawptr
```

Creates the file (mode 0644) or truncates it.


### `file_open_append`

```marrow
fn file_open_append(path: *u8) -> rawptr
```

### `file_close`

```marrow
fn file_close(handle: rawptr)
```

### `file_read`

```marrow
fn file_read(handle: rawptr, buf: *u8, count: i64) -> i64
```

Reads up to `count` bytes. Returns the number read, 0 at end of file, or a negative error.


### `file_write`

```marrow
fn file_write(handle: rawptr, buf: *u8, count: i64) -> i64
```

Writes every byte. Returns 0 on success.


### `file_get_size`

```marrow
fn file_get_size(handle: rawptr) -> i64
```

### `file_exists`

```marrow
fn file_exists(path: *u8) -> i32
```

### `file_remove`

```marrow
fn file_remove(path: *u8) -> i32
```

### `dir_create`

```marrow
fn dir_create(path: *u8) -> i32
```

### `read_to_string`

```marrow
fn read_to_string(path: *u8) -> *String
```

Reads the whole file (works for pipes too, where the size is unknown).


### `finish_read`

```marrow
fn finish_read(f: rawptr, content: *String, done: i64) -> *String
```

### `write_string_to_file`

```marrow
fn write_string_to_file(path: *u8, content: *String) -> i32
```

