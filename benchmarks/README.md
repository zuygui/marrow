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
| Recursive Fibonacci | 36 | **28** | 49 | 57 | 101 | 153 |
| Sieve of Eratosthenes, 5M | 37 | **32** | 41 | 66 | 120 | 148 |
| Longest Collatz chain below 1M | 36 | **28** | 35 | 62 | 105 | 146 |
| Mandelbrot, 400x400, f64 | 38 | **30** | 33 | 61 | 108 | 150 |
| Quicksort of 1M integers | 39 | **33** | 48 | 73 | 142 | 158 |
| 200x200 matrix product, f64 | 36 | **32** | 50 | 69 | 126 | 147 |

Run time of the compiled programs (`--exec`, median of 7 runs):

| Program | Marrow | gcc -O0 | gcc -O2 | clang -O2 | rustc -O | go build |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Recursive Fibonacci | 55 | 63 | **14** | 24 | 25 | 42 |
| Sieve of Eratosthenes, 5M | 28 | 51 | **24** | 26 | 34 | 27 |
| Longest Collatz chain below 1M | 253 | 363 | 184 | **134** | 147 | 232 |
| Mandelbrot, 400x400, f64 | 14 | 22 | 9 | **9** | 10 | 10 |
| Quicksort of 1M integers | 96 | 128 | 80 | **76** | 87 | 90 |
| 200x200 matrix product, f64 | 6 | 18 | 4 | **3** | **3** | 6 |

gcc 13.3.0, clang 18.1.3, rustc 1.97.0, go 1.24.7, Marrow v0.5.0.

## Caveats

- Marrow is **not** the fastest to build: `gcc -O0` is quicker on every program. It is on par with `gcc -O2`, ahead of `clang -O2`, and about three times quicker than `rustc` and `go build`. The tools do very different amounts of work (optimization, borrow checking, linking a runtime), so this is not a like-for-like quality comparison.
- The programs are tiny. Times are dominated by fixed costs: for Marrow, roughly
  7 ms for the Marrow front end, 14 ms for QBE and 23 ms for the system
  assembler and linker.
- Run time is measured with `--exec`: Marrow's output is faster than `gcc -O0` and in the range of Go, behind optimizing compilers: there is no inlining and no loop optimization yet.
- Results on a shared 2-core VM are noisy; compare ratios, not absolute values.
