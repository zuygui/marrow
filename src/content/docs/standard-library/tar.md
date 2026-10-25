---
title: "Tar"
description: Extract tar archives held in memory.
sidebar:
  order: 62
---

Source: `std/tar.mw`.

Extracts a tar archive that is already in memory. It reads the ustar format, with pax and GNU long names, which is what `git archive` and the code-hosting sites produce. Combined with [`gunzip`](/marrow/standard-library/inflate/), it unpacks a `.tar.gz` download without any external tool.

```marrow
@import("std/tar.mw")
@import("std/inflate.mw")

var raw: *String = gunzip(data, len);
var commit: *u8 = null;
var count: i64 = tar_extract(raw.data, raw.len, "/dest", 1, &commit);
if count < 0 {
    println(tar_error());
}
```

## Functions

| Function | Description |
|---|---|
| `tar_extract(data: *u8, n: i64, dest: *u8, strip: i64, commit: **u8) -> i64` | Extracts the archive into `dest` and returns the number of files, or `-1` on error. |
| `tar_error() -> *u8` | The message of the last error. |

`strip` leading path components are removed from every entry: `1` drops the top folder of a repository archive. `*commit` receives the commit id that `git archive` stores in the global pax header, when there is one.

## Safety

Absolute paths, `..` components and backslashes are refused, so an archive cannot write outside `dest`. Only regular files and directories are created: links and device files are skipped.
