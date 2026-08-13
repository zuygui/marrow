---
title: "fs — file I/O"
description: Opening, reading and writing files from std/fs.mw.
sidebar:
  order: 7
---

Source: `std/fs.mw`. Imports [`string.mw`](/marrow/standard-library/string/) and [`io.mw`](/marrow/standard-library/io/).

A thin, buffered-I/O wrapper around the C `<stdio.h>` file API (`fopen`/`fclose`/`fseek`/`ftell`/`fread`/`fwrite`). File handles are plain `rawptr` values (the underlying `FILE*`) — there's no dedicated `File` struct.

## Foreign bindings

| Function | Binds to (C) |
|---|---|
| `c_fopen(path: *u8, mode: *u8) -> rawptr` | `fopen` |
| `c_fclose(stream: rawptr) -> i32` | `fclose` |
| `c_fseek(stream: rawptr, offset: i64, whence: i32) -> i32` | `fseek` |
| `c_ftell(stream: rawptr) -> i64` | `ftell` |
| `c_fread(ptr: rawptr, size: i64, nmemb: i64, stream: rawptr) -> i64` | `fread` |
| `c_fwrite(ptr: rawptr, size: i64, nmemb: i64, stream: rawptr) -> i64` | `fwrite` |

## Functions

### `file_open_read(path: *u8) -> rawptr`
Opens `path` for **binary reading** (`fopen(path, "rb")`). Returns a null pointer on failure (file doesn't exist, permissions, etc.).

### `file_open_write(path: *u8) -> rawptr`
Opens (creating or truncating) `path` for **binary writing** (`fopen(path, "wb")`).

### `file_close(handle: rawptr)`
Closes a handle previously returned by `file_open_read`/`file_open_write`. Safe to call with a null handle (it's a no-op).

### `file_get_size(handle: rawptr) -> i64`
Returns the total size in bytes of an open file, by seeking to the end (`SEEK_END`), reading the position (`ftell`), then seeking back to the start (`SEEK_SET`). Returns `0` for a null handle.

### `read_to_string(path: *u8) -> *String`
Opens `path`, reads its **entire contents** into a freshly-allocated [`String`](/marrow/standard-library/string/) (see the string module), closes the file, and returns it. On any failure (file doesn't open, allocation fails) it prints a diagnostic to stderr (prefixed `[ERREUR FS]`, French for "FS ERROR") and returns a null pointer — always check the result before using it. The resulting `String` is null-terminated, so its `.data` field can also be passed anywhere a `*u8` C-string is expected.

### `write_string_to_file(path: *u8, content: *String) -> i32`
Opens (or creates/truncates) `path` for writing and writes the full contents of a `String` to it. Returns `1` on success, `0` if `content` is null or the file couldn't be opened (in which case it also prints a diagnostic to stderr). An empty string (`len == 0`) is treated as success without performing an actual write.

## Example

```marrow
@import("fs.mw")

fn copy_file(src: *u8, dst: *u8) -> i32 {
    var content: *String = read_to_string(src);
    if (content == 0) {
        ret 0;
    }

    var ok: i32 = write_string_to_file(dst, content);
    string_free(content);
    ret ok;
}
```
