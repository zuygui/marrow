---
title: The platform layer
description: How the standard library talks to the operating system, and what each platform supports.
sidebar:
  order: 2
---

Everything in the standard library that touches the outside world (files, sockets, clocks, memory, randomness) goes through a small set of functions called the **platform layer**. It lives in `std/platform/` and is selected at compile time with the target: `os.mw` contains `@import("platform/{target}")`, which expands to e.g. `platform/linux_x86_64`.

```
std/platform/
  linux_common.mw     shared Linux code
  linux_x86_64.mw     syscall numbers for each architecture
  linux_aarch64.mw
  linux_riscv64.mw
  macos_common.mw     BSD syscalls shared by both macOS architectures
  macos_x86_64.mw
  macos_aarch64.mw
  windows_x86_64.mw   Win32 / Winsock through a function table
```

## The primitive

```marrow
@extern("__marrow_syscall")
fn syscall(n: i64, a: i64, b: i64, c: i64, d: i64, e: i64, f: i64) -> i64;
```

The compiler provides `__marrow_syscall` in assembly for each target (see [Targets](/marrow/guides/cross-compilation/)). On Linux it is the kernel call; on macOS the BSD call; on Windows `n` selects a Win32 function from a table.

## The contract

Every platform file defines the same functions, so library code never needs to know the target:

- **Errors are `-errno`.** Success returns `0` or a count/descriptor; failure returns a negative errno value (`-2` = not found, `-13` = permission denied, ...).
- **`-38` (ENOSYS)** means the platform does not implement that call. Higher-level modules turn it into their usual failure value.
- Descriptors are plain integers. On Windows they carry Win32 handles and sockets.

| Group | Functions |
|---|---|
| Files | `sys_open`, `sys_read`, `sys_write`, `sys_close`, `sys_lseek`, `sys_unlink`, `sys_rename`, `sys_fstat_info` |
| Directories | `sys_mkdir`, `sys_rmdir`, `sys_getdents`, `sys_getcwd`, `sys_chdir` |
| Memory | `sys_mmap_anon`, `sys_munmap`, `sys_mprotect` |
| Process | `sys_exit`, `sys_getpid`, `sys_system` |
| Time & randomness | `sys_clock_ns`, `sys_sleep_ns`, `sys_getrandom` |
| Sockets | `sys_socket`, `sys_connect`, `sys_bind`, `sys_listen`, `sys_accept`, `sys_send`, `sys_recv`, `sys_sendto`, `sys_recvfrom`, `sys_setsockopt`, `sys_shutdown`, `sys_getsockname`, `sys_sockaddr_in` |
| Readiness | `sys_poll` |

The platform files also define the constants the portable code needs (`O_RDONLY`, `AF_INET`, `SO_RCVTIMEO`, ... and `ARCH`, which the [task](/marrow/guides/async/) module uses to build a new stack frame).

Where an operating system's data layout differs from Linux's, the platform file converts it, so portable code only ever sees one layout: macOS directory records are rewritten into Linux's `dirent64` format, Windows `FindFirstFile` results are presented the same way, and Winsock's `DWORD` socket timeouts are fed from the `timeval` the portable code builds.

## Support matrix

| Area | Linux | macOS | Windows |
|---|---|---|---|
| Files, directories, `stat` | yes | yes | yes |
| Time and sleep | `clock_gettime`, `nanosleep` | `gettimeofday` (the "monotonic" clock is the wall clock), `poll` | `QueryPerformanceCounter`, `Sleep` |
| Randomness | `getrandom` | `getentropy` | `RtlGenRandom` |
| Sockets (TCP/UDP) | yes | yes | Winsock |
| `sys_poll` | `ppoll` | `poll` | `WSAPoll` (sockets only) |
| `sys_system` (run a command) | yes | no | no |
| Guard pages (`sys_mprotect`) | yes | yes | yes |

## Adding a platform

1. Add `std/platform/<os>_<arch>.mw` with the functions above (copy the closest existing file).
2. Teach `Target::parse` and `Target::runtime_asm` in `src/runtime.rs` about it: the `__marrow_syscall` stub, the context switch, and, for freestanding targets, the entry point.
3. Add the target to the matrix in `.github/workflows/test-build.yaml`.

## Stability

The Linux layer talks to a stable kernel interface. macOS system call numbers are not guaranteed by Apple across releases, and Windows has no system call interface at all, which is why that layer calls documented Win32 functions through a table. The macOS and Windows layers are experimental.
