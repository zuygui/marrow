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
python3 benchmarks/run.py --exec           # also times the compiled programs
python3 benchmarks/chart.py                # redraws the README chart
python3 benchmarks/run.py --runs 5 --only fib,sieve
python3 benchmarks/run.py --marrow ./marrow
```

## Method

- Compile metric: wall-clock time of **one compiler invocation** that produces a native
  executable. With `--exec` the run time of the compiled programs is recorded too.
- One warm-up run, then the **median** of N runs (default 15).
- Every run appends a unique comment to the source and removes previous outputs,
  so no compiler cache can be hit.
- Toolchains: `marrow` (QBE backend, `cc` = gcc), `gcc -O0`, `gcc -O2`,
  `clang -O2`, `rustc -O`, `go build`.
- Machine, versions and date are recorded in `results.json` (`meta`).

## Results (Intel Xeon @ 2.10GHz, 2 cores, median ms, lower is better)

Compile time (one compiler invocation producing an executable):

| Program | Marrow | gcc -O0 | gcc -O2 | clang -O2 | rustc -O | go build |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Recursive Fibonacci | 45 | **32** | 60 | 63 | 106 | 150 |
| Sieve of Eratosthenes, 5M | 45 | **38** | 52 | 67 | 130 | 156 |
| Longest Collatz chain below 1M | 46 | **35** | 37 | 59 | 106 | 149 |
| Mandelbrot, 400x400, f64 | 47 | **31** | 43 | 66 | 115 | 156 |
| Quicksort of 1M integers | 59 | **42** | 58 | 84 | 157 | 153 |
| 200x200 matrix product, f64 | 46 | **37** | 59 | 76 | 144 | 163 |

Run time of the compiled programs (`--exec`, median of 7 runs):

| Program | Marrow | gcc -O0 | gcc -O2 | clang -O2 | rustc -O | go build |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Recursive Fibonacci | 76 | 80 | **20** | 32 | 32 | 57 |
| Sieve of Eratosthenes, 5M | 28 | 54 | **24** | 26 | 30 | 29 |
| Longest Collatz chain below 1M | 504 | 462 | 239 | **170** | 185 | 315 |
| Mandelbrot, 400x400, f64 | 20 | 28 | **13** | **13** | 13 | 16 |
| Quicksort of 1M integers | 134 | 175 | 108 | **105** | 109 | 112 |
| 200x200 matrix product, f64 | 8 | 23 | 7 | **4** | 4 | 8 |

gcc 13.3.0, clang 18.1.3, rustc 1.97.0, go 1.24.7, Marrow v0.5.0.

## Caveats

- Marrow is **not** the fastest: `gcc -O0` is quicker on every program. Against
  optimizing builds it is mixed: a few ms ahead of or behind `gcc -O2`, ahead
  of `clang -O2`, and two to three and a half times quicker than `rustc` and `go build`.
  The tools do very different amounts of work (optimization, borrow checking,
  linking a runtime), so this is not a like-for-like quality comparison.
- The programs are tiny. Times are dominated by fixed costs: for Marrow, roughly
  7 ms for the Marrow front end, 14 ms for QBE and 23 ms for the system
  assembler and linker.
- Run time is measured with `--exec`: Marrow's output is about as fast as `gcc -O0`, well behind optimizing compilers. QBE does no inlining and few loop optimizations.
- Results on a shared 2-core VM are noisy; compare ratios, not absolute values.
