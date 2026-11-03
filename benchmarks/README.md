# Benchmarks

Compile-time comparison between Marrow and other toolchains on six small
programs. Each program is implemented identically in Marrow, C, Rust and Go and
prints the same output in all four languages.

| Program | Description |
| --- | --- |
| `fib` | Recursive Fibonacci, n = 35 |
| `sieve` | Sieve of Eratosthenes up to 5M |
| `collatz` | Longest Collatz chain below 1M |
| `mandelbrot` | Mandelbrot set, 400x400, f64 |
| `sort` | Quicksort of 1M integers |
| `matmul` | 200x200 matrix product, f64 |

## Running

```sh
python3 benchmarks/run.py                  # 15 runs per toolchain, writes results.json
python3 benchmarks/run.py --runs 5 --only fib,sieve
python3 benchmarks/run.py --marrow ./marrow
```

## Method

- Metric: wall-clock time of **one compiler invocation** that produces a native
  executable (not the runtime of the compiled program).
- One warm-up run, then the **median** of N runs (default 15).
- Every run appends a unique comment to the source and removes previous outputs,
  so no compiler cache can be hit.
- Toolchains: `marrow` (QBE backend, `cc` = gcc), `gcc -O0`, `gcc -O2`,
  `clang -O2`, `rustc -O`, `go build`.
- Machine, versions and date are recorded in `results.json` (`meta`).

## Results (Intel Xeon @ 2.10GHz, 2 cores, median ms, lower is better)

| Program | Marrow | gcc -O0 | gcc -O2 | clang -O2 | rustc -O | go build |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Recursive Fibonacci | 51 | 36 | 74 | 70 | 116 | 173 |
| Sieve of Eratosthenes, 5M | 50 | 42 | 49 | 76 | 128 | 165 |
| Longest Collatz chain below 1M | 51 | 37 | 46 | 75 | 149 | 171 |
| Mandelbrot, 400x400, f64 | 50 | 38 | 49 | 73 | 122 | 187 |
| Quicksort of 1M integers | 53 | 43 | 62 | 85 | 166 | 168 |
| 200x200 matrix product, f64 | 53 | 41 | 59 | 95 | 172 | 191 |

gcc 13.3.0, clang 18.1.3, rustc 1.97.0, go 1.24.7, Marrow v0.4.0.

## Caveats

- Marrow is **not** the fastest: `gcc -O0` is quicker on every program. Against
  optimizing builds it is mixed: a few ms ahead of or behind `gcc -O2`, ahead
  of `clang -O2`, and two to three and a half times quicker than `rustc` and `go build`.
  The tools do very different amounts of work (optimization, borrow checking,
  linking a runtime), so this is not a like-for-like quality comparison.
- The programs are tiny. Times are dominated by fixed costs: for Marrow, roughly
  7 ms for the Marrow front end, 14 ms for QBE and 23 ms for the system
  assembler and linker.
- Only compile time is measured. QBE does little optimization, so
  runtime performance of the produced binaries is not covered here.
- Results on a shared 2-core VM are noisy; compare ratios, not absolute values.
