//! Self-hosting checks: every component of the compiler that has been ported to
//! Marrow (under `compiler/`) is built with the Rust compiler and compared with
//! its Rust original on every `.mw` file of the repository.
//!
//! Currently:
//! - the lexer (`compiler/lexdump.mw` vs `marrow --dump-tokens`);
//! - the parser (`compiler/astdump.mw` vs `marrow --dump-ast`);
//! - the module loader (`compiler/importdump.mw` vs `marrow --dump-imports`).
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
    for d in ["std", "compiler", "tests/run", "tests/ui", "tests/lexer", "tests/parser", "tests/import", "tests/tls"] {
        let p = root.join(d);
        if p.exists() {
            collect(&p, &mut files);
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
