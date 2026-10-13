---
title: "Time"
description: Clocks, sleeping and calendar conversion.
sidebar:
  order: 28
---

Source: `std/time.mw`.

Clocks, sleeping and calendar conversion (proleptic Gregorian, UTC).


## Types

### `DateTime`

```marrow
struct DateTime {
    year: i64;
    month: i64;   // 1..12
    day: i64;     // 1..31
    hour: i64;
    minute: i64;
    second: i64;
    weekday: i64; // 0 = Sunday
    yday: i64;    // 0-based day of the year
}
```

## Functions

### `time_now_ns`

```marrow
fn time_now_ns() -> i64
```

Nanoseconds since 1970-01-01 00:00:00 UTC (negative on error).


### `time_mono_ns`

```marrow
fn time_mono_ns() -> i64
```

Nanoseconds from an arbitrary starting point; never goes backwards.


### `time_unix`

```marrow
fn time_unix() -> i64
```

### `sleep_ns`

```marrow
fn sleep_ns(ns: i64)
```

### `sleep_ms`

```marrow
fn sleep_ms(ms: i64)
```

### `is_leap_year`

```marrow
fn is_leap_year(y: i64) -> bool
```

### `days_in_month`

```marrow
fn days_in_month(y: i64, m: i64) -> i64
```

### `days_from_civil`

```marrow
fn days_from_civil(y: i64, m: i64, d: i64) -> i64
```

Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm).


### `datetime_from_unix`

```marrow
fn datetime_from_unix(t: i64) -> DateTime
```

Seconds since the epoch -> broken-down UTC time.


### `datetime_to_unix`

```marrow
fn datetime_to_unix(dt: DateTime) -> i64
```

### `time_pad2`

```marrow
fn time_pad2(buf: *u8, n: i64)
```

### `datetime_format_iso`

```marrow
fn datetime_format_iso(buf: *u8, dt: DateTime) -> i64
```

"YYYY-MM-DDTHH:MM:SSZ" (20 bytes, no terminator) -> returns 20.


