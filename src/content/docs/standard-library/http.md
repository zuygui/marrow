---
title: "HTTP"
description: HTTP/1.1 client and server, with HTTPS.
sidebar:
  order: 30
---

Source: `std/http.mw`.

The client speaks HTTP/1.1 and HTTP over TLS (see [`tls.mw`](/marrow/standard-library/tls/)); the server side gives you request parsing and reply helpers to build a small web server on top of `http_accept`.

HTTP/1.1 client and server on top of std/net.mw.

Server loop:

```marrow
var srv: i64 = http_listen(8080);
while 1 == 1 {
    var conn: *HttpConn = http_accept(srv);
    var req: *HttpMessage = http_read_request(conn);
    http_reply_text(conn, 200, "hello\n");
    http_conn_close(conn);
}
```

Client (http:// and https://; https verifies the certificate chain and the host name against the system trust store, see http\_set\_roots):

```marrow
var res: *HttpMessage = http_get("https://example.com/");
```

Messages, connections and URLs are heap allocated; use the \*\_free functions.


## Types

### `HttpConn`

```marrow
struct HttpConn {
    fd: i64;
    buf: *u8;
    len: i64;
    pos: i64;
    keep_alive: i64;
    tls: *Tls;
}
```

### `HttpMessage`

```marrow
struct HttpMessage {
    method: *u8;      // requests
    target: *u8;      // requests: path and query, as sent
    version: *u8;
    status: i64;      // responses
    reason: *u8;
    headers: *VecPtr; // name, value, name, value, ...
    body: *u8;        // always NUL terminated
    body_len: i64;
}
```

### `Url`

```marrow
struct Url {
    scheme: *u8;
    host: *u8;
    port: i64;
    path: *u8;  // path and query, always starts with '/'
}
```

## Functions

### `http_conn_new`

```marrow
fn http_conn_new(fd: i64) -> *HttpConn
```

### `http_conn_write`

```marrow
fn http_conn_write(c: *HttpConn, data: *u8, n: i64) -> i64
```

Writes all bytes (through TLS when the connection has it); 0 on success.


### `http_conn_close`

```marrow
fn http_conn_close(c: *HttpConn)
```

### `http_conn_fill`

```marrow
fn http_conn_fill(c: *HttpConn) -> bool
```

Refills the buffer; false at end of stream or error.


### `http_conn_read_line`

```marrow
fn http_conn_read_line(c: *HttpConn, out: *String, limit: i64) -> bool
```

Reads one line (without the line terminator) into `out`. False if the stream ended first or the line is longer than `limit`.


### `http_conn_read_exact`

```marrow
fn http_conn_read_exact(c: *HttpConn, dst: *u8, n: i64) -> bool
```

Copies exactly `n` bytes (from the buffer, then the socket) into `dst`.


### `http_message_new`

```marrow
fn http_message_new() -> *HttpMessage
```

### `http_message_free`

```marrow
fn http_message_free(m: *HttpMessage)
```

### `http_header_add`

```marrow
fn http_header_add(m: *HttpMessage, name: *u8, value: *u8)
```

### `http_header_get`

```marrow
fn http_header_get(m: *HttpMessage, name: *u8) -> *u8
```

First header called `name` (case-insensitive), or null.


### `http_header_has_token`

```marrow
fn http_header_has_token(m: *HttpMessage, name: *u8, token: *u8) -> bool
```

### `http_set_body`

```marrow
fn http_set_body(m: *HttpMessage, data: *u8, len: i64)
```

### `http_read_headers`

```marrow
fn http_read_headers(c: *HttpConn, m: *HttpMessage) -> bool
```

Reads "Name: value" lines until the blank line.


### `http_read_chunked`

```marrow
fn http_read_chunked(c: *HttpConn, m: *HttpMessage, max_body: i64) -> bool
```

### `http_read_body`

```marrow
fn http_read_body(c: *HttpConn, m: *HttpMessage, until_close: bool, max_body: i64) -> bool
```

Reads the body announced by the headers. `until_close` allows bodies that end when the peer closes the connection (responses only).


### `http_max_body`

```marrow
fn http_max_body() -> i64
```

### `http_read_request`

```marrow
fn http_read_request(c: *HttpConn) -> *HttpMessage
```

Reads one request. Null when the client closed the connection or sent something malformed.


### `http_read_response`

```marrow
fn http_read_response(c: *HttpConn, head_only: bool) -> *HttpMessage
```

Reads one response. `head_only` is for replies to HEAD requests.


### `http_reason`

```marrow
fn http_reason(status: i64) -> *u8
```

### `http_reply`

```marrow
fn http_reply(c: *HttpConn, status: i64, content_type: *u8, extra_headers: *u8, body: *u8, body_len: i64) -> i64
```

Sends a full response. `extra_headers` is raw text such as "Cache-Control: no-store\\r\\n" (or null). Content-Length and Connection are added automatically.


### `http_reply_text`

```marrow
fn http_reply_text(c: *HttpConn, status: i64, text: *u8) -> i64
```

### `http_reply_html`

```marrow
fn http_reply_html(c: *HttpConn, status: i64, html: *u8) -> i64
```

### `http_reply_json`

```marrow
fn http_reply_json(c: *HttpConn, status: i64, json: *u8) -> i64
```

### `http_reply_redirect`

```marrow
fn http_reply_redirect(c: *HttpConn, status: i64, location: *u8) -> i64
```

### `url_encode`

```marrow
fn url_encode(s: *u8) -> *u8
```

Percent-encodes everything except unreserved characters (RFC 3986).


### `url_decode`

```marrow
fn url_decode(s: *u8, plus_is_space: bool) -> *u8
```

Decodes %XX sequences; `plus_is_space` is for query strings and forms.


### `http_path`

```marrow
fn http_path(target: *u8) -> *u8
```

The path part of a request target (before `?`), as a new string.


### `http_query_param`

```marrow
fn http_query_param(target: *u8, name: *u8) -> *u8
```

Value of query parameter `name` (decoded, new string) or null.


### `url_free`

```marrow
fn url_free(u: *Url)
```

### `url_parse`

```marrow
fn url_parse(url: *u8) -> *Url
```

Parses http(s)://host[:port][/path?query]. Null if malformed.


### `http_listen`

```marrow
fn http_listen(port: i64) -> i64
```

Listens on every interface.


### `http_accept`

```marrow
fn http_accept(listener: i64) -> *HttpConn
```

Waits for a client; null on error. Reads time out after 15 seconds.


### `http_mime_for`

```marrow
fn http_mime_for(path: *u8) -> *u8
```

### `http_serve_file`

```marrow
fn http_serve_file(c: *HttpConn, root: *u8, target: *u8) -> bool
```

Serves `root` + the request path. Paths containing ".." are refused, and "/" maps to index.html. Returns true if a file was sent.


### `http_last_error`

```marrow
fn http_last_error() -> *u8
```

Reason for the last failed client call.


### `http_set_roots`

```marrow
fn http_set_roots(roots: *VecPtr)
```

### `http_roots`

```marrow
fn http_roots() -> *VecPtr
```

### `http_request`

```marrow
fn http_request(method: *u8, url: *u8, extra_headers: *u8, body: *u8, body_len: i64) -> *HttpMessage
```

Sends one request and reads the response. `extra_headers` is raw header text ending in "\\r\\n" (or null); `body` may be null. Handles `http://` and `https://` URLs.


### `http_get`

```marrow
fn http_get(url: *u8) -> *HttpMessage
```

### `http_post`

```marrow
fn http_post(url: *u8, content_type: *u8, body: *u8, body_len: i64) -> *HttpMessage
```

### `http_get_follow`

```marrow
fn http_get_follow(url: *u8, max_redirects: i64) -> *HttpMessage
```

GET that follows up to `max_redirects` redirections (301, 302, 303, 307, 308).


