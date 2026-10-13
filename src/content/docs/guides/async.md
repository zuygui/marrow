---
title: Async tasks
description: Cooperative tasks, channels and non-blocking I/O with std/task.mw.
sidebar:
  order: 1
---

`std/task.mw` adds concurrency to Marrow without threads, signals or any runtime outside the standard library. It provides **stackful coroutines** ("tasks") scheduled cooperatively on one thread, an event loop built on `poll(2)`, and channels to pass values between tasks.

```marrow
@import("std/task.mw")
@import("std/io.mw")

fn worker(arg: rawptr) -> i64 {
    task_sleep_ms(10);
    println("worker done");
    ret 7;
}

@export fn main(argc: i32, argv: rawptr) -> i32 {
    var t: *Task = task_spawn(worker, null);
    task_run();                     // runs until every task finished or got stuck
    println_i64(task_join(t));      // 7
    ret 0;
}
```

## How it works

- A task is a function `fn(rawptr) -> i64` (a [function pointer](/marrow/language/types/#function-pointer-types)) plus its own stack, allocated with `mmap` (256 KiB by default) and fenced by a guard page: overflowing it faults instead of corrupting memory. Use `task_spawn_sized` for a different size.
- Switching tasks is a handful of instructions: a small assembly routine saves the callee-saved registers, swaps stack pointers and returns into the other task. It is implemented for x86_64, aarch64 and riscv64.
- A task runs until it **yields**. Tasks give up the CPU only in `task_yield`, `task_sleep_*`, `task_wait_fd`, `task_join` and the channel and I/O helpers. A task that loops forever without calling one of them blocks every other task.
- `task_run()` is the scheduler. It runs ready tasks round-robin; when none is ready it sleeps in `poll(2)` until a timer expires or a file descriptor becomes ready. It returns when all tasks have finished, or when the remaining ones can never wake up (for example, all blocked on an empty channel nobody will fill).

## Waiting for I/O

`task_wait_fd(fd, POLL_IN)` suspends the current task until the descriptor is readable; `POLL_OUT` waits for writability. The helpers wrap the common cases:

| Function | Waits for |
|---|---|
| `task_accept(listener)` | an incoming connection |
| `task_recv(fd, buf, len)` / `task_read` | data to read |
| `task_write_all(fd, buf, len)` | the socket to accept every byte |

An echo server then reads like straight-line code, while every connection is served concurrently:

```marrow
fn echo(arg: rawptr) -> i64 {
    var fd: i64 = cast(i64) arg;
    var buf: *u8 = alloc(4096);
    var n: i64 = task_recv(fd, buf, 4096);
    while n > 0 {
        task_write_all(fd, buf, n);
        n = task_recv(fd, buf, 4096);
    }
    net_close(fd);
    dealloc(buf);
    ret 0;
}

fn server(arg: rawptr) -> i64 {
    var listener: i64 = tcp_listen(8080, 64);
    while true {
        var c: i64 = task_accept(listener);
        task_detach(task_spawn(echo, cast(rawptr) c));
    }
}
```

`task_detach` frees a task's bookkeeping as soon as it ends; otherwise the memory is released by `task_join`.

## Channels

A `Chan` is a bounded queue of `i64` values (use `cast` to send pointers). `chan_send` blocks while the channel is full and `chan_recv` blocks while it is empty. After `chan_close`, `chan_send` returns `-1` and `chan_recv` drains what is left and then returns `-1`.

```marrow
var ch: *Chan = chan_new(4);
chan_send(ch, 42);
var v: i64 = 0;
chan_recv(ch, &v);      // v == 42
chan_close(ch);
chan_free(ch);
```

## Joining

`task_join(t)` returns the task's result. Inside a task it suspends the caller until `t` finishes; from the main program (outside any task) it first runs the scheduler to completion.

## Platform notes

| Platform | Status |
|---|---|
| Linux x86_64 | Tested in CI, including the network tests. |
| Linux aarch64, riscv64 | Assembled and linked; the CI runs them under qemu-user. |
| macOS | Uses `poll` and `mprotect`. Assembled in CI; not yet run. |
| Windows | Sockets are polled with `WSAPoll`; only sockets can be waited on (not files or the console). Experimental; not yet run. |

See the [`task` reference](/marrow/standard-library/task/) for every function and the [platform layer](/marrow/standard-library/platform/) for the system calls behind it.
