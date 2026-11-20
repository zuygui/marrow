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
| Recursive Fibonacci | 33 | **31** | 58 | 57 | 103 | 156 |
| Sieve of Eratosthenes, 5M | **36** | **36** | 41 | 70 | 130 | 158 |
| Longest Collatz chain below 1M | 34 | **33** | 48 | 63 | 118 | 168 |
| Mandelbrot, 400x400, f64 | **36** | **36** | 43 | 69 | 112 | 160 |
| Quicksort of 1M integers | **40** | 42 | 56 | 89 | 167 | 180 |
| 200x200 matrix product, f64 | **42** | 46 | 54 | 90 | 153 | 158 |

Run time of the compiled programs (`--exec`, median of 7 runs):

| Program | Marrow | gcc -O0 | gcc -O2 | clang -O2 | rustc -O | go build |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Recursive Fibonacci | 31 | 62 | **18** | 26 | 28 | 46 |
| Sieve of Eratosthenes, 5M | **27** | 52 | **27** | 30 | 30 | 29 |
| Longest Collatz chain below 1M | 312 | 449 | 235 | **178** | 181 | 285 |
| Mandelbrot, 400x400, f64 | 18 | 25 | 13 | 11 | **9** | 11 |
| Quicksort of 1M integers | 114 | 154 | **87** | 102 | 123 | 114 |
| 200x200 matrix product, f64 | 11 | 29 | 6 | **3** | **3** | 7 |

gcc 13.3.0, clang 18.1.3, rustc 1.97.0, go 1.24.7, Marrow v0.5.0.

## Caveats

- Marrow builds about as fast as `gcc -O0` (mean 37 ms each, within the noise), quicker than `gcc -O2` and `clang -O2`, and about four times quicker than `rustc` and `go build`. The tools do very different amounts of work (optimization, borrow checking, linking a runtime), so this is not a like-for-like quality comparison.
- The programs are tiny. Times are dominated by fixed costs: for Marrow, most of the
  time goes to QBE and to the system assembler and linker rather than to the
  front end.
- Run time is measured with `--exec`: Marrow's output is faster than `gcc -O0` and in the range of Go, behind optimizing compilers: small functions are inlined, but there is no loop optimization yet.
- Results on a shared 2-core VM are noisy; compare ratios, not absolute values.
