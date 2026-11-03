#!/usr/bin/env python3
"""Compile-time benchmark: the same programs built to a native executable by Marrow, C, Rust and Go.

    python3 benchmarks/run.py [--runs N] [--out results.json]

Each timing is the wall-clock time of ONE compiler invocation that produces an executable
(compile + assemble + link). Before every run a unique comment is appended to the source and
all build products are removed, so no compiler can answer from a cache. The first run of every
(program, toolchain) pair is a warm-up and is not counted; the reported value is the median.
"""
import argparse, json, os, platform, shutil, statistics, subprocess, sys, tempfile, time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)

PROGRAMS = [
    ("fib", "fib(35)", "Recursive Fibonacci"),
    ("sieve", "sieve", "Sieve of Eratosthenes, 5M"),
    ("collatz", "collatz", "Longest Collatz chain below 1M"),
    ("mandelbrot", "mandelbrot", "Mandelbrot, 400x400, f64"),
    ("sort", "quicksort", "Quicksort of 1M integers"),
    ("matmul", "matmul", "200x200 matrix product, f64"),
]

def sh(cmd, cwd):
    return subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)

def version(cmd):
    try:
        out = subprocess.run(cmd, capture_output=True, text=True).stdout.strip().splitlines()
        return out[0] if out else "?"
    except OSError:
        return None

def toolchains(marrow):
    # id, label, extension, command(src, out)
    return [
        ("marrow", "Marrow", "mw", lambda s, o: [marrow, s, o + ".ssa"]),
        ("gcc-O0", "gcc -O0", "c", lambda s, o: ["gcc", "-O0", "-o", o, s]),
        ("gcc-O2", "gcc -O2", "c", lambda s, o: ["gcc", "-O2", "-o", o, s]),
        ("clang-O2", "clang -O2", "c", lambda s, o: ["clang", "-O2", "-o", o, s]),
        ("rustc-O", "rustc -O", "rs", lambda s, o: ["rustc", "-O", "-o", o, s]),
        ("go", "go build", "go", lambda s, o: ["go", "build", "-o", o, s]),
    ]

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--runs", type=int, default=15)
    ap.add_argument("--out", default=os.path.join(HERE, "results.json"))
    ap.add_argument("--marrow", default=os.path.join(ROOT, "build", "marrow"))
    ap.add_argument("--only", default="")
    a = ap.parse_args()
    marrow = os.path.abspath(a.marrow)
    tcs = toolchains(marrow)
    only = [x for x in a.only.split(",") if x]
    cpu = "?"
    try:
        for line in open("/proc/cpuinfo"):
            if line.startswith("model name"):
                cpu = line.split(":", 1)[1].strip()
                break
    except OSError:
        cpu = platform.processor()
    meta = {
        "date": time.strftime("%Y-%m-%d"),
        "cpu": cpu,
        "cores": os.cpu_count(),
        "os": platform.platform(),
        "runs": a.runs,
        "versions": {
            "marrow": version([marrow, "--version"]),
            "gcc": version(["gcc", "--version"]),
            "clang": version(["clang", "--version"]),
            "rustc": version(["rustc", "--version"]),
            "go": version(["go", "version"]),
        },
        "method": "wall-clock time of one compiler invocation building an executable, median of N runs after one warm-up; unique source each run; cold outputs",
    }
    results = []
    work = tempfile.mkdtemp(prefix="marrow-bench-")
    os.symlink(os.path.join(ROOT, "std"), os.path.join(work, "std"))
    try:
        for pid, short, label in PROGRAMS:
            entry = {"id": pid, "name": short, "label": label, "toolchains": []}
            loc = {}
            for tid, tlabel, ext, cmd in tcs:
                if only and tid not in only:
                    continue
                src = os.path.join(HERE, f"{pid}.{ext}")
                text = open(src).read()
                loc[tid] = len([l for l in text.splitlines() if l.strip()])
                times = []
                for n in range(a.runs + 1):
                    d = os.path.join(work, f"{pid}-{tid}")
                    shutil.rmtree(d, ignore_errors=True)
                    os.makedirs(d)
                    os.symlink(os.path.join(ROOT, "std"), os.path.join(d, "std"))
                    s = os.path.join(d, f"{pid}.{ext}")
                    open(s, "w").write(text + f"\n// run {n} {time.time_ns()}\n")
                    o = os.path.join(d, pid)
                    t0 = time.perf_counter()
                    r = sh(cmd(s, o), d)
                    dt = (time.perf_counter() - t0) * 1000
                    if r.returncode != 0 or not os.path.exists(o):
                        sys.exit(f"{tid} failed on {pid}:\n{r.stderr or r.stdout}")
                    if n > 0:
                        times.append(dt)
                entry["toolchains"].append({
                    "id": tid, "label": tlabel,
                    "median_ms": round(statistics.median(times), 1),
                    "min_ms": round(min(times), 1),
                    "max_ms": round(max(times), 1),
                    "source_lines": loc[tid],
                })
                print(f"{pid:11} {tlabel:10} median {statistics.median(times):8.1f} ms  (min {min(times):.1f}, max {max(times):.1f})", flush=True)
            results.append(entry)
    finally:
        shutil.rmtree(work, ignore_errors=True)
    json.dump({"meta": meta, "programs": results}, open(a.out, "w"), indent=2)
    open(a.out, "a").write("\n")
    print("wrote", a.out)

if __name__ == "__main__":
    main()
