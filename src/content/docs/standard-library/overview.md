---
title: Standard Library overview
description: What ships in std/, how it's organized, and how to import it.
sidebar:
  order: 1
---

The standard library lives in the `std/` directory of the repository and is written **entirely in Marrow** (`.mw` files). It does not use libc: every module sits on a small [platform layer](/marrow/standard-library/platform/) that talks to the operating system directly (Linux, macOS and Windows; x86_64, aarch64 and riscv64), so programs can be linked fully static with `--nostdlib`.

Every file in `std/` starts with `@no_main`, which marks it as a library rather than a standalone program (see [Modules & decorators](/marrow/language/decorators-and-modules/)).

## Importing it

```marrow
@import("std/std.mw")
```

`std.mw` is an umbrella that imports the core modules (`io`, `mem`, `string`, `text`, `vec`, `map`, `fmt`, `raw`, `sys`, `fs`, `env`, `path`, `dir`). The larger or more specialised modules are imported on demand:

```marrow
@import("std/std.mw")
@import("std/http.mw")      // HTTP and HTTPS
@import("std/json.mw")
@import("std/task.mw")      // async tasks
```

Modules import each other where they depend on one another, and importing the same file twice is always safe (see the deduplication rule on the modules page).

## Modules


### Core

| Module | |
|---|---|
| [`io.mw`](/marrow/standard-library/io/) | Printing to stdout and stderr. |
| [`mem.mw`](/marrow/standard-library/mem/) | Allocation, copying and arenas. |
| [`heap.mw`](/marrow/standard-library/heap/) | The libc-free allocator behind `alloc`. |
| [`raw.mw`](/marrow/standard-library/raw/) | Byte-level copy, fill and compare primitives. |
| [`string.mw`](/marrow/standard-library/string/) | C-string helpers and the growable `String`. |
| [`text.mw`](/marrow/standard-library/text/) | Character classes, searching, parsing and splitting. |
| [`fmt.mw`](/marrow/standard-library/fmt/) | Number formatting into buffers. |
| [`vec.mw`](/marrow/standard-library/vec/) | Growable arrays. |
| [`map.mw`](/marrow/standard-library/map/) | Hash maps and linear maps. |
| [`sys.mw`](/marrow/standard-library/sys/) | Exit, panic and assert. |
| [`os.mw`](/marrow/standard-library/os/) | Raw system calls and the platform layer entry point. |
| [`env.mw`](/marrow/standard-library/env/) | Command-line arguments and environment variables. |

### Files & paths

| Module | |
|---|---|
| [`fs.mw`](/marrow/standard-library/fs/) | Reading and writing files. |
| [`dir.mw`](/marrow/standard-library/dir/) | Directories: listing, creating, walking. |
| [`path.mw`](/marrow/standard-library/path/) | Path joining and splitting. |

### Math & time

| Module | |
|---|---|
| [`math.mw`](/marrow/standard-library/math/) | Floating-point and integer math, written from scratch. |
| [`rand.mw`](/marrow/standard-library/rand/) | Random numbers. |
| [`time.mw`](/marrow/standard-library/time/) | Clocks, sleeping and calendar conversion. |

### Networking

| Module | |
|---|---|
| [`net.mw`](/marrow/standard-library/net/) | TCP, UDP, DNS and IPv4 helpers. |
| [`http.mw`](/marrow/standard-library/http/) | HTTP/1.1 client and server, with HTTPS. |
| [`tls.mw`](/marrow/standard-library/tls/) | A TLS 1.3 client. |
| [`x509.mw`](/marrow/standard-library/x509/) | Certificate parsing and chain verification. |

### Crypto & encoding

| Module | |
|---|---|
| [`hash.mw`](/marrow/standard-library/hash/) | SHA-1/2, HMAC, HKDF and PBKDF2. |
| [`chacha.mw`](/marrow/standard-library/chacha/) | The ChaCha20-Poly1305 AEAD. |
| [`aes.mw`](/marrow/standard-library/aes/) | AES-128-GCM. |
| [`x25519.mw`](/marrow/standard-library/x25519/) | Elliptic-curve Diffie-Hellman on Curve25519. |
| [`bigint.mw`](/marrow/standard-library/bigint/) | Arbitrary-precision unsigned integers. |
| [`rsa.mw`](/marrow/standard-library/rsa/) | RSA signature verification. |
| [`ecdsa.mw`](/marrow/standard-library/ecdsa/) | ECDSA signature verification on P-256 and P-384. |
| [`asn1.mw`](/marrow/standard-library/asn1/) | A DER reader. |
| [`encoding.mw`](/marrow/standard-library/encoding/) | Hex, Base64 and UTF-8. |

### Data formats

| Module | |
|---|---|
| [`json.mw`](/marrow/standard-library/json/) | A JSON parser and writer. |

### Concurrency

| Module | |
|---|---|
| [`task.mw`](/marrow/standard-library/task/) | Cooperative tasks, channels and async I/O. |

## Conventions across the library

- **Errors are values.** There are no exceptions and no `Result` type. Functions report failure with a null pointer, `false`, or a negative return value (system-call wrappers return `-errno`). Genuinely unrecoverable situations call `panic()`, which prints to stderr and exits with code 1.
- **Manual memory.** Every `_new` function that allocates has a matching `_free`. Nothing allocates behind your back; the arena in [`mem.mw`](/marrow/standard-library/mem/) is an opt-in tool.
- **C strings.** Text is `*u8` ending in a zero byte unless a function takes an explicit length. Byte buffers take a pointer and a length.
- **No generics.** Containers come in concrete flavours (`VecPtr`, `VecI64`...).
- **Platform differences stay inside the platform layer.** Portable modules compile unchanged for every target; calls a platform cannot support return `-38` (ENOSYS).
- **Security notes.** The cryptographic code is meant for verification and key agreement in TLS and similar uses. Comparisons of secrets use `ct_equal`; the rest of the code is variable-time.
