//! Self-hosting checks: every component of the compiler that has been ported to
//! Marrow (under `compiler/`) is built with the Rust compiler and compared with
//! its Rust original on every `.mw` file of the repository.
//!
//! Currently: the lexer (`compiler/lexdump.mw` vs `marrow --dump-tokens`).
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

#[test]
fn lexer_matches_rust_lexer() {
    if !cfg!(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64"))) || !have("qbe") || !have("cc") {
        eprintln!("skipping self-hosting tests: need Linux, `qbe` and `cc`");
        return;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let work = root.join("target/selfhost");
    let _ = fs::remove_dir_all(&work);
    copy_dir(&root.join("std"), &work.join("std"));
    copy_dir(&root.join("compiler"), &work.join("compiler"));

    let marrow = env!("CARGO_BIN_EXE_marrow");
    let build = Command::new(marrow)
        .current_dir(work.join("compiler"))
        .args(["--nostdlib", "--color", "never", "lexdump.mw"])
        .output()
        .unwrap();
    assert!(build.status.success(), "lexdump failed to build:\n{}", String::from_utf8_lossy(&build.stderr));
    let lexdump = work.join("compiler/lexdump");

    let mut files = Vec::new();
    for d in ["std", "compiler", "tests/run", "tests/ui", "tests/lexer", "tests/tls"] {
        let p = root.join(d);
        if p.exists() {
            collect(&p, &mut files);
        }
    }
    assert!(files.len() > 50, "found only {} files", files.len());

    let mut failures = Vec::new();
    for f in &files {
        let rust = Command::new(marrow).arg("--dump-tokens").arg(f).output().unwrap();
        let mine = Command::new(&lexdump).arg(f).output().unwrap();
        if rust.stdout != mine.stdout {
            let a = String::from_utf8_lossy(&rust.stdout).to_string();
            let b = String::from_utf8_lossy(&mine.stdout).to_string();
            let line = a.lines().zip(b.lines()).position(|(x, y)| x != y).unwrap_or(0);
            failures.push(format!(
                "{}: first difference at line {}\n  rust:    {:?}\n  marrow:  {:?}",
                f.strip_prefix(root).unwrap().display(),
                line + 1,
                a.lines().nth(line),
                b.lines().nth(line)
            ));
        }
    }
    assert!(failures.is_empty(), "{} of {} files differ:\n{}", failures.len(), files.len(), failures.join("\n"));
}
