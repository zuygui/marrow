---
title: "Inflate"
description: gzip and DEFLATE decompression.
sidebar:
  order: 61
---

Source: `std/inflate.mw`.

Decompresses gzip files (RFC 1952) and raw DEFLATE streams (RFC 1951). It follows the small `puff` decoder by Mark Adler: it favors being short and easy to check over speed. The package manager uses it to unpack the archives it downloads, together with [`std/tar.mw`](/marrow/standard-library/tar/).

```marrow
@import("std/inflate.mw")

var out: *String = gunzip(data, len);   // null when the data is not valid gzip
if out != null {
    println(out.data);
}
```

## Functions

| Function | Description |
|---|---|
| `gunzip(data: *u8, n: i64) -> *String` | Decompresses a gzip file (one member). Returns `null` when the data is not valid gzip. |
| `inflate_raw(data: *u8, n: i64) -> *String` | Decompresses a raw DEFLATE stream. Returns `null` on error. |
