---
title: "OS"
description: Raw system calls and the platform layer entry point.
sidebar:
  order: 21
---

Source: `std/os.mw`.

The thin layer between the standard library and the operating system. The functions `sys_read`, `sys_write`, `sys_mmap_anon` and so on come from [the platform layer](/marrow/standard-library/platform/).

Raw Linux system calls, no libc involved. The numbers and the `syscall` stub are picked per target (`--target`, default: the host). Compile programs that use only this module with `marrow --nostdlib`.


## Functions

### `os_strlen`

```marrow
fn os_strlen(s: *u8) -> u64
```

### `os_write_all`

```marrow
fn os_write_all(fd: i32, buf: *u8, len: i64) -> i64
```

Writes the whole buffer, retrying on partial writes. Returns 0 on success.


### `os_print`

```marrow
fn os_print(s: *u8)
```

### `os_println`

```marrow
fn os_println(s: *u8)
```

### `os_eprint`

```marrow
fn os_eprint(s: *u8)
```

