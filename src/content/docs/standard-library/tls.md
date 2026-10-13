---
title: "TLS"
description: A TLS 1.3 client.
sidebar:
  order: 31
---

Source: `std/tls.mw`.

A TLS 1.3 client with X25519, ChaCha20-Poly1305 and AES-128-GCM. The server certificate chain and host name are always verified against the anchors you pass in; `HelloRetryRequest` is not supported. Load the system trust store with `x509_load_system_roots`.

TLS 1.3 client (RFC 8446).

- cipher suites TLS\_CHACHA20\_POLY1305\_SHA256, TLS\_AES\_128\_GCM\_SHA256
- key exchange  X25519
- server auth   X.509 chain verified against the trust store and the host name, CertificateVerify with ECDSA (P-256/P-384) or RSA-PSS, and the Finished MAC.  There is no way to skip verification.

Usage:

```marrow
var fd: i64 = tcp_connect(host, 443);
var roots: *VecPtr = vec_ptr_new(16);  x509_load_system_roots(roots);
var t: *Tls = tls_connect(fd, host, roots);   // null on failure
tls_write(t, data, n);  tls_read(t, buf, cap);  tls_close(t);
```


## Types

### `Tls`

```marrow
struct Tls {
    fd: i64;
    c_key: *u8;
    c_iv: *u8;
    c_seq: i64;
    s_key: *u8;
    s_iv: *u8;
    s_seq: i64;
    c_secret: *u8;
    s_secret: *u8;
    encrypted: i64;
    th: *u8;
    thlen: i64;
    thcap: i64;
    hs: *u8;
    hslen: i64;
    hscap: i64;
    rbuf: *u8;
    rpos: i64;
    rlen: i64;
    eof: i64;
    failed: i64;
    suite: i64;
}
```

## Functions

### `tls_last_error`

```marrow
fn tls_last_error() -> *u8
```

### `tls_connect`

```marrow
fn tls_connect(fd: i64, host: *u8, anchors: *VecPtr) -> *Tls
```

Runs the TLS 1.3 handshake on a connected socket.  `anchors` is a VecPtr of trusted \*Cert (see x509\_load\_system\_roots).  Returns null on failure (see tls\_last\_error).  On failure the socket is left open for the caller.


### `tls_free`

```marrow
fn tls_free(t: *Tls)
```

### `tls_handshake`

```marrow
fn tls_handshake(t: *Tls, host: *u8, anchors: *VecPtr) -> bool
```

### `tls_write`

```marrow
fn tls_write(t: *Tls, buf: *u8, n: i64) -> i64
```

Sends `n` bytes; returns n, or a negative value on error.


### `tls_read`

```marrow
fn tls_read(t: *Tls, buf: *u8, cap: i64) -> i64
```

Reads up to `cap` bytes.  Returns 0 at end of stream (close\_notify or closed connection), a negative value on error.


### `tls_close`

```marrow
fn tls_close(t: *Tls)
```

Sends close\_notify.  Does not close the socket.


