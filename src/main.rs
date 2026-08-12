use std::{env, fs, path::PathBuf};

mod ast;
mod codegen;
mod error;
mod import;
mod lexer;
mod parser;
mod token;

fn main() {
    let mut args = env::args().skip(1);
    let input_path = match args.next() {
        Some(path) => {
            if path == "--version" {
                let version = env!("CARGO_PKG_VERSION");
                println!("Marrow version: v{}", version);
                std::process::exit(0);
            }
            PathBuf::from(path)
        },
        None => {
            eprintln!("Usage: marrow <input.mw> [output.ssa]");
            std::process::exit(2);
        }
    };

    if !input_path.exists() {
        eprintln!("input file not found: {}", input_path.display());
        std::process::exit(2);
    }

    let output_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        let mut path = input_path.clone();
        path.set_extension("ssa");
        path
    });

    let (program, is_library) = import::load_with_imports(&input_path).unwrap_or_else(|err| {
        eprintln!("{err}");
        std::process::exit(1);
    });

    let qbe_il = codegen::generate(&program).unwrap_or_else(|err| {
        eprintln!("code generation failed: {err}");
        std::process::exit(1);
    });

    fs::write(&output_path, &qbe_il).unwrap_or_else(|err| {
        eprintln!("failed to write {}: {err}", output_path.display());
        std::process::exit(2);
    });

    let qbe_output_path = output_path.with_extension("s");
    let qbe_status = std::process::Command::new("qbe")
        .arg(&output_path)
        .arg("-o")
        .arg(&qbe_output_path)
        .status()
        .unwrap_or_else(|err| {
            eprintln!("failed to execute qbe: {err}");
            std::process::exit(2);
        });

    if !qbe_status.success() {
        eprintln!("qbe failed with exit code: {}", qbe_status.code().unwrap_or(-1));
        std::process::exit(1);
    }

    let compiler = if cfg!(target_os = "windows") {
        "gcc"
    } else if cfg!(target_os = "macos") {
        "cc"
    } else if cfg!(target_os = "linux") {
        "cc"
    } else {
        eprintln!("Unsupported OS for compilation. Only QBE IL will be generated.");
        return;
    };

    if is_library {
        let object_output_path = input_path.with_extension("o");
        let compiler_status = std::process::Command::new(compiler)
            .arg("-c")
            .arg(&qbe_output_path)
            .arg("-o")
            .arg(&object_output_path)
            .status()
            .unwrap_or_else(|err| {
                eprintln!("failed to execute {}: {err}", compiler);
                std::process::exit(2);
            });

        if !compiler_status.success() {
            eprintln!("{} failed with exit code: {}", compiler, compiler_status.code().unwrap_or(-1));
            std::process::exit(1);
        }

        eprintln!(
            "Library compiled (no 'main', see '@no_main'): {}\nLink it into a program with: cc your_program.o {} -o your_program",
            object_output_path.display(),
            object_output_path.display()
        );
        return;
    }

    let executable_output_path = input_path.with_extension("");
    let compiler_status = std::process::Command::new(compiler)
        .arg(&qbe_output_path)
        .arg("-o")
        .arg(&executable_output_path)
        .status()
        .unwrap_or_else(|err| {
            eprintln!("failed to execute {}: {err}", compiler);
            std::process::exit(2);
        });

    if !compiler_status.success() {
        eprintln!("{} failed with exit code: {}", compiler, compiler_status.code().unwrap_or(-1));
        std::process::exit(1);
    }

    eprintln!(
        "Compilation successful! Executable created at: {}",
        executable_output_path.display()
    );
}