---
title: "Net"
description: TCP, UDP, DNS and IPv4 helpers.
sidebar:
  order: 29
---

Source: `std/net.mw`.

Sockets are plain file descriptors held in `i64`. IPv4 addresses are packed into a `u32`; build one with `ip4(a, b, c, d)`.

TCP and UDP over IPv4, plus a small DNS resolver. No libc: every function is built on the socket calls of the platform layer.

Addresses are `u32` values in host order (a.b.c.d -> a&lt;&lt;24 | b&lt;&lt;16 | c&lt;&lt;8 | d). Functions returning `i64` give a file descriptor or a negative errno.


## Types

### `Timeval`

```marrow
struct Timeval {
    sec: i64;
    usec: i64;
}
```

## Functions

### `ip4`

```marrow
fn ip4(a: i64, b: i64, c: i64, d: i64) -> u32
```

### `ip4_octet`

```marrow
fn ip4_octet(ip: u32, index: i64) -> i64
```

### `ip4_parse`

```marrow
fn ip4_parse(s: *u8, out: *u32) -> bool
```

Parses "a.b.c.d" (exactly four decimal parts, each 0..255).


### `ip4_format`

```marrow
fn ip4_format(buf: *u8, ip: u32) -> i64
```

"a.b.c.d" into `buf` (at most 15 bytes); returns the length.


### `net_close`

```marrow
fn net_close(fd: i64)
```

### `net_set_timeout_ms`

```marrow
fn net_set_timeout_ms(fd: i64, ms: i64) -> i64
```

### `net_set_nodelay`

```marrow
fn net_set_nodelay(fd: i64) -> i64
```

### `net_sockaddr`

```marrow
fn net_sockaddr(buf: *u8, ip: u32, port: i64)
```

### `tcp_connect_ip`

```marrow
fn tcp_connect_ip(ip: u32, port: i64) -> i64
```

### `tcp_listen_on`

```marrow
fn tcp_listen_on(ip: u32, port: i64, backlog: i64) -> i64
```

Listens on `ip:port` (port 0 picks a free one; see `net_local_port`).


### `tcp_listen`

```marrow
fn tcp_listen(port: i64, backlog: i64) -> i64
```

All interfaces.


### `tcp_accept`

```marrow
fn tcp_accept(fd: i64, peer: *u32) -> i64
```

Accepts one connection. `peer` (may be null) receives the client address.


### `net_local_port`

```marrow
fn net_local_port(fd: i64) -> i64
```

Port a socket is bound to (host order), or a negative errno.


### `net_read`

```marrow
fn net_read(fd: i64, buf: *u8, n: i64) -> i64
```

Reads up to `n` bytes (0 means the peer closed the connection).


### `net_write_all`

```marrow
fn net_write_all(fd: i64, buf: *u8, n: i64) -> i64
```

Sends every byte. Returns 0 on success or a negative errno.


### `net_write_str`

```marrow
fn net_write_str(fd: i64, s: *u8) -> i64
```

### `net_shutdown_write`

```marrow
fn net_shutdown_write(fd: i64)
```

Stops sending (the peer then reads end-of-file) but keeps reading open.


### `udp_socket`

```marrow
fn udp_socket() -> i64
```

### `udp_send_to`

```marrow
fn udp_send_to(fd: i64, ip: u32, port: i64, buf: *u8, n: i64) -> i64
```

### `udp_recv`

```marrow
fn udp_recv(fd: i64, buf: *u8, n: i64) -> i64
```

### `be16_at`

```marrow
fn be16_at(buf: *u8, i: i64) -> i64
```

### `dns_skip_name`

```marrow
fn dns_skip_name(buf: *u8, len: i64, i: i64) -> i64
```

Skips a (possibly compressed) domain name; returns the index after it.


### `dns_build_query`

```marrow
fn dns_build_query(buf: *u8, name: *u8, id: i64) -> i64
```

Builds an A query for `name` into `buf`; returns its length or -1.


### `dns_parse_response`

```marrow
fn dns_parse_response(buf: *u8, len: i64, id: i64, out: *u32) -> bool
```

Extracts the first IPv4 address of a response to our query.


### `dns_nameserver`

```marrow
fn dns_nameserver(out: *u32) -> bool
```

First `nameserver` line of /etc/resolv.conf that is an IPv4 address.


### `dns_from_hosts`

```marrow
fn dns_from_hosts(name: *u8, out: *u32) -> bool
```

Looks `name` up in /etc/hosts (IPv4 entries only).


### `dns_resolve`

```marrow
fn dns_resolve(name: *u8, out: *u32) -> bool
```

Resolves a host name or dotted address to IPv4. Tries, in order: literal address, "localhost", /etc/hosts, then a UDP query to the first IPv4 nameserver of /etc/resolv.conf.


### `tcp_connect`

```marrow
fn tcp_connect(host: *u8, port: i64) -> i64
```

Resolves `host` and connects.


