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
//! - shortest round-trip `f64` printing (`compiler/floatfmt.mw` vs Rust's `{}`);
//! - exact decimal -> `f64` parsing (`compiler/floatparse.mw` vs `str::parse::<f64>`);
//! - the code generator (`compiler/qbedump.mw` vs `marrow --dump-qbe`).
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
    for d in ["std", "compiler", "tests/run", "tests/ui", "tests/lexer", "tests/parser", "tests/import", "tests/typeck", "tests/codegen", "tests/tls"] {
        let p = root.join(d);
        if p.exists() {
            collect(&p, &mut files);
        }
    }
    if tool == "lowerdump" || tool == "checkdump" || tool == "qbedump" {
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

#[test]
fn codegen_matches_rust_codegen() {
    compare("qbedump", "--dump-qbe", true);
}

/// `compiler/floatparse.mw` rounds decimal literals exactly like Rust's `str::parse`.
#[test]
fn float_parsing_matches_rust() {
    if !cfg!(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64"))) || !have("qbe") || !have("cc") {
        eprintln!("skipping self-hosting tests: need Linux, `qbe` and `cc`");
        return;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let work = root.join("target").join("selfhost-floatparsedump");
    let _ = fs::remove_dir_all(&work);
    copy_dir(&root.join("std"), &work.join("std"));
    copy_dir(&root.join("compiler"), &work.join("compiler"));
    let marrow = env!("CARGO_BIN_EXE_marrow");
    let build = Command::new(marrow)
        .current_dir(work.join("compiler"))
        .args(["--nostdlib", "--color", "never"])
        .arg("floatparsedump.mw")
        .output()
        .unwrap();
    assert!(build.status.success(), "floatparsedump failed to build:\n{}", String::from_utf8_lossy(&build.stderr));

    let mut lines: Vec<String> = [
        "0", "0.0", "1", "0.1", "1e5", "1.5e-3", "9007199254740993", "9007199254740992.5", "2.2250738585072011e-308",
        "2.2250738585072014e-308", "1.7976931348623157e308", "1.7976931348623158e308", "1.7976931348623159e308",
        "5e-324", "2e-324", "2.4703282292062327e-324", "2.4703282292062328e-324", "1e400", "1e-400", "0.000001",
        "123456789012345678901234567890", "0.1000000000000000055511151231257827021181583404541015625",
        "8.5e-5", "7.0e-10", "4.35", "1e23", "8.41e21", "5.708990770823839e-300",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let mut s: u64 = 0x2545F4914F6CDD1D;
    let mut next = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    for _ in 0..4000 {
        let nd = 1 + next() % 25;
        let digits: String = (0..nd).map(|_| char::from(b'0' + (next() % 10) as u8)).collect();
        let dot = (next() % (nd + 1)) as usize;
        let mut lit = format!("{}.{}", &digits[..dot], &digits[dot..]);
        if lit.starts_with('.') {
            lit.insert(0, '0');
        }
        if lit.ends_with('.') {
            lit.push('0');
        }
        if next() % 3 != 0 {
            lit.push_str(&format!("e{}", (next() % 640) as i64 - 330));
        }
        lines.push(lit);
    }
    // doubles written out in full and half-way cases between neighbours
    for _ in 0..2000 {
        let v = f64::from_bits(next() & 0x7fef_ffff_ffff_ffff);
        lines.push(format!("{:e}", v));
        let w = f64::from_bits(v.to_bits() + 1);
        if w.is_finite() && ((v + w) / 2.0).is_finite() {
            lines.push(format!("{:e}", (v + w) / 2.0));
        }
    }
    let mut input = String::new();
    let mut expected = String::new();
    for l in &lines {
        input.push_str(l);
        input.push('\n');
        expected.push_str(&format!("{}\n", l.parse::<f64>().unwrap()));
    }
    let input_path = work.join("lits.txt");
    fs::write(&input_path, input).unwrap();
    let out = Command::new(work.join("compiler").join("floatparsedump")).arg(&input_path).output().unwrap();
    let got = String::from_utf8_lossy(&out.stdout).to_string();
    assert_eq!(expected.lines().count(), got.lines().count(), "line count differs");
    let diffs: Vec<_> = expected
        .lines()
        .zip(got.lines())
        .zip(lines.iter())
        .filter(|((a, b), _)| a != b)
        .take(10)
        .map(|((a, b), l)| format!("{l}: rust {a}, marrow {b}"))
        .collect();
    assert!(diffs.is_empty(), "float parsing differs:\n{}", diffs.join("\n"));
}

/// The whole compiler written in Marrow (`compiler/marrow.mw`) bootstraps: the Rust
/// compiler builds it (stage 0), stage 0 builds it again (stage 1), stage 1 builds it once
/// more (stage 2). The QBE IL and the assembly of the three builds must be identical, and
/// the stage 2 compiler must compile and run a real program.
#[test]
fn compiler_bootstraps_to_a_fixed_point() {
    if !cfg!(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64"))) || !have("qbe") || !have("cc") {
        eprintln!("skipping self-hosting tests: need Linux, `qbe` and `cc`");
        return;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let work = root.join("target").join("selfhost-bootstrap");
    let _ = fs::remove_dir_all(&work);
    for stage in ["s0", "s1", "s2"] {
        copy_dir(&root.join("std"), &work.join(stage).join("std"));
        copy_dir(&root.join("compiler"), &work.join(stage).join("compiler"));
    }
    let build = |compiler: &Path, stage: &str| {
        let dir = work.join(stage).join("compiler");
        let out = Command::new(compiler)
            .current_dir(&dir)
            .args(["--nostdlib", "marrow.mw"])
            .output()
            .unwrap();
        assert!(out.status.success(), "{stage} failed to build:\n{}", String::from_utf8_lossy(&out.stderr));
        dir
    };
    let s0 = build(Path::new(env!("CARGO_BIN_EXE_marrow")), "s0");
    let s1 = build(&s0.join("marrow"), "s1");
    let s2 = build(&s1.join("marrow"), "s2");
    for ext in ["ssa", "s"] {
        let a = fs::read(s0.join(format!("marrow.{ext}"))).unwrap();
        let b = fs::read(s1.join(format!("marrow.{ext}"))).unwrap();
        let c = fs::read(s2.join(format!("marrow.{ext}"))).unwrap();
        assert!(a == b, ".{ext} differs between the Rust-built and the self-built compiler");
        assert!(b == c, ".{ext} differs between stage 1 and stage 2");
    }

    // The self-built compiler produces a working program.
    let case = work.join("case");
    copy_dir(&root.join("std"), &case.join("std"));
    let src = root.join("tests/run/aes.mw");
    let expected = fs::read(root.join("tests/run/aes.stdout")).unwrap();
    fs::copy(&src, case.join("aes.mw")).unwrap();
    let out = Command::new(s2.join("marrow")).current_dir(&case).args(["--nostdlib", "aes.mw"]).output().unwrap();
    assert!(out.status.success(), "stage 2 failed to compile aes.mw:\n{}", String::from_utf8_lossy(&out.stderr));
    let run = Command::new(case.join("aes")).output().unwrap();
    assert!(run.stdout == expected, "the program built by stage 2 prints something else");

    // The self-built compiler renders every diagnostic exactly like the Rust one,
    // with and without colors.
    let diag = work.join("diag");
    copy_dir(&root.join("std"), &diag.join("std"));
    let mut files = Vec::new();
    for sub in ["ui", "typeck", "lexer", "parser", "import", "codegen"] {
        let from = root.join("tests").join(sub);
        if from.is_dir() {
            copy_dir(&from, &diag.join("tests").join(sub));
            collect(&diag.join("tests").join(sub), &mut files);
        }
    }
    files.sort();
    assert!(files.len() > 50, "found only {} diagnostic cases", files.len());
    let rust = Path::new(env!("CARGO_BIN_EXE_marrow"));
    for file in &files {
        let rel = file.strip_prefix(&diag).unwrap();
        for color in ["never", "always"] {
            let run = |bin: &Path| Command::new(bin).current_dir(&diag).args(["--check", "--color", color]).arg(rel).output().unwrap();
            let a = run(rust);
            let b = run(&s2.join("marrow"));
            assert!(
                a.stderr == b.stderr && a.status.code() == b.status.code(),
                "diagnostics differ for {} (--color {color}):\n--- rust\n{}\n--- marrow\n{}",
                rel.display(),
                String::from_utf8_lossy(&a.stderr),
                String::from_utf8_lossy(&b.stderr)
            );
        }
    }
}
