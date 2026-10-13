---
title: "Task"
description: Cooperative tasks, channels and async I/O.
sidebar:
  order: 43
---

Source: `std/task.mw`.

Stackful coroutines with a single-threaded scheduler. See the [async guide](/marrow/guides/async/) for a walkthrough.

Cooperative tasks (stackful coroutines) with a single-threaded scheduler.

```marrow
fn worker(arg: rawptr) -> i64 { task_sleep_ms(10); ret 7; }
var t: *Task = task_spawn(worker, null);
task_run();                // runs every task until none can make progress
var r: i64 = task_join(t); // result of `worker`
```

A task gives up the CPU only in task\_yield, task\_sleep\_\*, task\_wait\_fd, task\_join and the channel operations. Waiting for I/O uses poll(2); on platforms without it (Windows, for now) sleeping and yielding still work.


## Constants

| Name | Type | Value | Notes |
|---|---|---|---|
| `TASK_STACK_SIZE` | `i64` | `262144` |  |
| `TASK_READY` | `i64` | `1` |  |
| `TASK_RUNNING` | `i64` | `2` |  |
| `TASK_SLEEP` | `i64` | `3` |  |
| `TASK_FDWAIT` | `i64` | `4` |  |
| `TASK_BLOCKED` | `i64` | `5` |  |
| `TASK_DONE` | `i64` | `6` |  |
| `POLL_IN` | `i64` | `1` |  |
| `POLL_OUT` | `i64` | `4` |  |
| `POLL_ERR` | `i64` | `8` |  |
| `POLL_HUP` | `i64` | `16` |  |

## Types

### `Task`

```marrow
struct Task {
    sp: i64;
    state: i64;
    entry: fn(rawptr) -> i64;
    arg: rawptr;
    stack: i64;
    stack_size: i64;
    next: *Task;
    wake_ns: i64;
    fd: i64;
    events: i64;
    revents: i64;
    joiners: *Task;
    result: i64;
    detached: i64;
}
```

### `Chan`

```marrow
struct Chan {
    buf: *i64;
    cap: i64;
    len: i64;
    head: i64;
    closed: i64;
    recvq: *Task;
    sendq: *Task;
}
```

## Functions

### `task_spawn_sized`

```marrow
fn task_spawn_sized(f: fn(rawptr) -> i64, arg: rawptr, size: i64) -> *Task
```

### `task_spawn`

```marrow
fn task_spawn(f: fn(rawptr) -> i64, arg: rawptr) -> *Task
```

### `task_detach`

```marrow
fn task_detach(t: *Task)
```

The task's memory is released when it ends instead of when it is joined.


### `task_yield`

```marrow
fn task_yield()
```

### `task_sleep_ns`

```marrow
fn task_sleep_ns(ns: i64)
```

### `task_sleep_ms`

```marrow
fn task_sleep_ms(ms: i64)
```

### `task_wait_fd`

```marrow
fn task_wait_fd(fd: i64, events: i64) -> i64
```

Blocks until `fd` is ready for `events` (POLL\_IN / POLL\_OUT); returns the revents.


### `task_join`

```marrow
fn task_join(t: *Task) -> i64
```

Waits for the task to end and returns its result. From outside a task this first runs the scheduler until every task has finished or is stuck.


### `task_run`

```marrow
fn task_run()
```

Runs the scheduler until every task is done or the remaining ones can never wake up (blocked on a channel or join nobody will satisfy).


### `task_read`

```marrow
fn task_read(fd: i64, buf: *u8, len: u64) -> i64
```

### `task_write_all`

```marrow
fn task_write_all(fd: i64, buf: *u8, len: i64) -> i64
```

Writes the whole buffer. Returns 0 on success, -errno otherwise.


### `task_accept`

```marrow
fn task_accept(fd: i64) -> i64
```

### `task_recv`

```marrow
fn task_recv(fd: i64, buf: *u8, len: u64) -> i64
```

### `chan_new`

```marrow
fn chan_new(cap: i64) -> *Chan
```

### `chan_free`

```marrow
fn chan_free(c: *Chan)
```

### `chan_close`

```marrow
fn chan_close(c: *Chan)
```

### `chan_send`

```marrow
fn chan_send(c: *Chan, v: i64) -> i64
```

Blocks while the channel is full. Returns 0, or -1 when it is closed.


### `chan_recv`

```marrow
fn chan_recv(c: *Chan, out: *i64) -> i64
```

Blocks while the channel is empty. Returns 0, or -1 once it is closed and drained.


