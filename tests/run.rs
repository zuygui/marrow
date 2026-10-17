//! End-to-end tests: every `tests/run/*.mw` is compiled with `--nostdlib`,
//! executed, and its standard output compared with the `.stdout` file next to
//! it. The exit status must be 0.
//!
//! Needs `qbe` and `cc` in PATH and a Linux x86_64 or aarch64 host; the test
//! quietly passes when they are missing so `cargo test` works anywhere.

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

#[test]
fn run_programs() {
    if !cfg!(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64"))) || !have("qbe") || !have("cc") {
        eprintln!("skipping end-to-end tests: need Linux, `qbe` and `cc`");
        return;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let work = root.join("target/e2e");
    let _ = fs::remove_dir_all(&work);
    copy_dir(&root.join("std"), &work.join("std"));

    let mut files: Vec<_> = fs::read_dir(root.join("tests/run")).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "mw")).collect();
    files.sort();
    let mut failures = Vec::new();

    for file in files {
        let name = file.file_stem().unwrap().to_str().unwrap().to_string();
        let src = work.join(format!("{name}.mw"));
        fs::copy(&file, &src).unwrap();
        let build = Command::new(env!("CARGO_BIN_EXE_marrow")).current_dir(&work).args(["--nostdlib", "--color", "never"]).arg(&src).output().unwrap();
        if !build.status.success() {
            failures.push(format!("{name}: compilation failed\n{}", String::from_utf8_lossy(&build.stderr)));
            continue;
        }
        let run = Command::new(work.join(&name)).current_dir(&work).output().unwrap();
        let expected = fs::read_to_string(file.with_extension("stdout")).unwrap_or_default();
        let actual = String::from_utf8_lossy(&run.stdout).to_string();
        if !run.status.success() || expected != actual {
            failures.push(format!("{name}: status {:?}\nexpected:\n{expected}\nactual:\n{actual}", run.status.code()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n---\n"));
}
