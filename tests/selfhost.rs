//! Self-hosting checks: every component of the compiler that has been ported to
//! Marrow (under `compiler/`) is built with the Rust compiler and compared with
//! its Rust original on every `.mw` file of the repository.
//!
//! Currently:
//! - the lexer (`compiler/lexdump.mw` vs `marrow --dump-tokens`);
//! - the parser (`compiler/astdump.mw` vs `marrow --dump-ast`);
//! - the module loader (`compiler/importdump.mw` vs `marrow --dump-imports`);
//! - the lowering pass (`compiler/lowerdump.mw` vs `marrow --dump-lowered`);
//! - the type checker (`compiler/checkdump.mw` vs `marrow --dump-check`).
//! - shortest round-trip `f64` printing (`compiler/floatfmt.mw` vs Rust's `{}`).
//!
//! Needs `qbe` and `cc` and a Linux x86_64 / aarch64 host, like `tests/run.rs`.

use std::{fs, path::Path, process::Command};

fn have(tool: &str) -> bool {
    Command::new(tool).arg("-h").output().is_ok() || Command::new(tool).arg("--version").output().is_ok()
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &dest);
        } else {
            fs::copy(entry.path(), dest).unwrap();
        }
    }
}

fn collect(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().map_or(false, |e| e == "mw") {
            out.push(p);
        }
    }
}

/// Builds `compiler/<tool>.mw` and compares its output with `marrow <flag> <file>` on every
/// `.mw` file of the repository (successful parses and error cases alike).
fn compare(tool: &str, flag: &str, with_target: bool) {
    if !cfg!(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64"))) || !have("qbe") || !have("cc") {
        eprintln!("skipping self-hosting tests: need Linux, `qbe` and `cc`");
        return;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    // One work directory per tool: the two tests run in parallel.
    let work = root.join("target").join(format!("selfhost-{tool}"));
    let _ = fs::remove_dir_all(&work);
    copy_dir(&root.join("std"), &work.join("std"));
    copy_dir(&root.join("compiler"), &work.join("compiler"));

    let marrow = env!("CARGO_BIN_EXE_marrow");
    let build = Command::new(marrow)
        .current_dir(work.join("compiler"))
        .args(["--nostdlib", "--color", "never"])
        .arg(format!("{tool}.mw"))
        .output()
        .unwrap();
    assert!(build.status.success(), "{tool} failed to build:\n{}", String::from_utf8_lossy(&build.stderr));
    let exe = work.join("compiler").join(tool);

    let mut files = Vec::new();
    for d in ["std", "compiler", "tests/run", "tests/ui", "tests/lexer", "tests/parser", "tests/import", "tests/typeck", "tests/tls"] {
        let p = root.join(d);
        if p.exists() {
            collect(&p, &mut files);
        }
    }
    if tool == "lowerdump" || tool == "checkdump" {
        // The program tests import `std/std.mw` relative to themselves: stage copies
        // next to the `std` directory so that they load and actually get lowered.
        for d in ["tests/run", "tests/ui"] {
            let mut cases = Vec::new();
            collect(&root.join(d), &mut cases);
            for c in cases {
                let dest = work.join(format!("{}_{}", d.replace('/', "_"), c.file_name().unwrap().to_string_lossy()));
                fs::copy(&c, &dest).unwrap();
                files.push(dest);
            }
        }
    }
    assert!(files.len() > 50, "found only {} files", files.len());

    let mut failures = Vec::new();
    for f in &files {
        let mut rust = Command::new(marrow);
        rust.arg(flag).arg(f);
        let mut mine = Command::new(&exe);
        mine.arg(f);
        if with_target {
            let target = if cfg!(target_arch = "aarch64") { "linux_aarch64" } else { "linux_x86_64" };
            rust.arg(target);
            mine.arg(target);
        }
        let rust = rust.output().unwrap();
        let mine = mine.output().unwrap();
        if rust.stdout != mine.stdout {
            let a = String::from_utf8_lossy(&rust.stdout).to_string();
            let b = String::from_utf8_lossy(&mine.stdout).to_string();
            let line = a.lines().zip(b.lines()).position(|(x, y)| x != y).unwrap_or(0);
            let cut = |l: Option<&str>| l.map(|l| l.chars().take(300).collect::<String>());
            failures.push(format!(
                "{}: first difference at line {}\n  rust:    {:?}\n  marrow:  {:?}",
                f.strip_prefix(root).unwrap().display(),
                line + 1,
                cut(a.lines().nth(line)),
                cut(b.lines().nth(line))
            ));
        }
    }
    assert!(failures.is_empty(), "{} of {} files differ:\n{}", failures.len(), files.len(), failures.join("\n"));
}

#[test]
fn lexer_matches_rust_lexer() {
    compare("lexdump", "--dump-tokens", false);
}

#[test]
fn parser_matches_rust_parser() {
    compare("astdump", "--dump-ast", false);
}

#[test]
fn import_loader_matches_rust_loader() {
    compare("importdump", "--dump-imports", true);
}

#[test]
fn lowering_matches_rust_lowering() {
    compare("lowerdump", "--dump-lowered", true);
}

#[test]
fn typeck_matches_rust_typeck() {
    compare("checkdump", "--dump-check", true);
}

/// `compiler/floatfmt.mw` prints `f64` values exactly like Rust's `{}` (shortest
/// round-trip form): fixed values, k/1000, scaled decimals and random bit patterns.
#[test]
fn float_formatting_matches_rust() {
    if !cfg!(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64"))) || !have("qbe") || !have("cc") {
        eprintln!("skipping self-hosting tests: need Linux, `qbe` and `cc`");
        return;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let work = root.join("target").join("selfhost-floatdump");
    let _ = fs::remove_dir_all(&work);
    copy_dir(&root.join("std"), &work.join("std"));
    copy_dir(&root.join("compiler"), &work.join("compiler"));
    let marrow = env!("CARGO_BIN_EXE_marrow");
    let build = Command::new(marrow)
        .current_dir(work.join("compiler"))
        .args(["--nostdlib", "--color", "never"])
        .arg("floatdump.mw")
        .output()
        .unwrap();
    assert!(build.status.success(), "floatdump failed to build:\n{}", String::from_utf8_lossy(&build.stderr));

    let mut values: Vec<f64> = vec![
        0.0, -0.0, 1.0, -1.0, 0.1, 0.5, 1.5, 100.0, 1e21, 1e-7, 123456789.125, f64::MAX, f64::MIN_POSITIVE, 5e-324,
        f64::EPSILON, f64::INFINITY, f64::NEG_INFINITY, f64::NAN, 9007199254740993.0, 0.30000000000000004,
    ];
    for k in 0..2000 {
        values.push(k as f64 / 1000.0);
        values.push(k as f64 * 1.0e-5);
        values.push(k as f64 * 1.0e15);
    }
    let mut s: u64 = 0x9E3779B97F4A7C15;
    for _ in 0..3000 {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        values.push(f64::from_bits(s));
    }
    let mut input = String::new();
    let mut expected = String::new();
    for v in &values {
        input.push_str(&format!("{:016x}\n", v.to_bits()));
        expected.push_str(&format!("{v}\n"));
    }
    let input_path = work.join("bits.txt");
    fs::write(&input_path, input).unwrap();
    let out = Command::new(work.join("compiler").join("floatdump")).arg(&input_path).output().unwrap();
    let got = String::from_utf8_lossy(&out.stdout).to_string();
    let diffs: Vec<_> = expected.lines().zip(got.lines()).enumerate().filter(|(_, (a, b))| a != b).take(10).collect();
    assert_eq!(expected.lines().count(), got.lines().count(), "line count differs");
    assert!(diffs.is_empty(), "float formatting differs: {diffs:?}");
}
