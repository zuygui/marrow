//! Golden tests: every `tests/ui/*.mw` is checked and its diagnostics are
//! compared with the neighbouring `.stderr` file.
//! Run with `BLESS=1 cargo test` to (re)generate the expected output.

use std::{fs, path::Path, process::Command};

#[test]
fn ui_diagnostics() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/ui");
    let bless = std::env::var_os("BLESS").is_some();
    let mut files: Vec<_> = fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "mw")).collect();
    files.sort();
    let mut failures = Vec::new();

    for file in files {
        let name = file.file_name().unwrap().to_str().unwrap().to_string();
        let out = Command::new(env!("CARGO_BIN_EXE_marrow"))
            .current_dir(&dir)
            .args(["--check", "--color", "never", &name])
            .output()
            .unwrap();
        let actual = String::from_utf8_lossy(&out.stderr).replace("\r\n", "\n");
        let expected_path = file.with_extension("stderr");
        if bless {
            fs::write(&expected_path, &actual).unwrap();
            continue;
        }
        let expected = fs::read_to_string(&expected_path).unwrap_or_default();
        if expected != actual {
            failures.push(format!("--- {name}\nexpected:\n{expected}\nactual:\n{actual}"));
        }
    }
    assert!(failures.is_empty(), "UI mismatches (BLESS=1 to update):\n{}", failures.join("\n"));
}
