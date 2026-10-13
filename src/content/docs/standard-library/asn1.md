---
title: "ASN.1"
description: A DER reader.
sidebar:
  order: 40
---

Source: `std/asn1.mw`.

A minimal, allocation-free reader for DER-encoded data.

Minimal strict DER reader.  All functions work on (pointer, length) windows and never read outside them.


## Constants

| Name | Type | Value | Notes |
|---|---|---|---|
| `ASN1_BOOLEAN` | `i64` | `1` |  |
| `ASN1_INTEGER` | `i64` | `2` |  |
| `ASN1_BIT_STRING` | `i64` | `3` |  |
| `ASN1_OCTET_STRING` | `i64` | `4` |  |
| `ASN1_NULL` | `i64` | `5` |  |
| `ASN1_OID` | `i64` | `6` |  |
| `ASN1_UTF8` | `i64` | `12` |  |
| `ASN1_PRINTABLE` | `i64` | `19` |  |
| `ASN1_IA5` | `i64` | `22` |  |
| `ASN1_UTCTIME` | `i64` | `23` |  |
| `ASN1_GENTIME` | `i64` | `24` |  |
| `ASN1_SEQUENCE` | `i64` | `48` |  |
| `ASN1_SET` | `i64` | `49` |  |

## Types

### `Der`

```marrow
struct Der {
    p: *u8;
    len: i64;
    pos: i64;
}
```

Cursor over a window: `pos` is the offset of the next element.


## Functions

### `der_tlv`

```marrow
fn der_tlv(p: *u8, avail: i64, tag: *i64, hdr: *i64, len: *i64) -> bool
```

Parses the TLV header at `p`.  On success stores the tag byte, the header size and the content length, and guarantees hdr + len &lt;= avail. Only single-byte tags and definite, minimally-encoded lengths are accepted.


### `der_init`

```marrow
fn der_init(d: *Der, p: *u8, len: i64)
```

### `der_done`

```marrow
fn der_done(d: *Der) -> bool
```

### `der_next`

```marrow
fn der_next(d: *Der, want: i64, val: *rawptr, vlen: *i64) -> bool
```

Reads the next element and checks its tag.  On success `*val`/`*vlen` point at the content and the cursor advances past it.


### `der_next_any`

```marrow
fn der_next_any(d: *Der, tag: *i64, val: *rawptr, vlen: *i64, all: *rawptr, alen: *i64) -> bool
```

Like der\_next but returns the whole TLV (header included) via `*all`/`*alen`, and the tag via `*tag`, for any tag.


### `der_peek`

```marrow
fn der_peek(d: *Der) -> i64
```

Peeks the tag of the next element (-1 at the end or on malformed input).


### `der_uint`

```marrow
fn der_uint(val: *u8, vlen: i64, out: *rawptr, outlen: *i64) -> bool
```

An INTEGER's magnitude without its sign-padding zero byte.


### `der_oid_is`

```marrow
fn der_oid_is(val: *u8, vlen: i64, want: *u8, wlen: i64) -> bool
```

Compares an OID's DER content against raw bytes.


