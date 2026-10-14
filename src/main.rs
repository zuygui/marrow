use std::{env, fs, path::PathBuf, process};

mod ast;
mod codegen;
mod diagnostic;
mod dump;
mod import;
mod lexer;
mod lower;
mod parser;
mod runtime;
mod token;
mod typeck;
mod types;

use runtime::Target;
use diagnostic::{ColorChoice, Diagnostic, Emitter, SourceMap};

const USAGE: &str = "\
Usage: marrow [OPTIONS] <input.mw> [output.ssa]

Options:
      --check            Type-check the program and stop (no code is generated)
      --nostdlib         Link without libc: use Marrow's own `_start` and syscall stub
      --target <T>       Target as <arch>-<os>, e.g. x86_64-linux, aarch64-linux, riscv64-linux
                         (default: the host; cross targets only emit assembly)
      --color <WHEN>     Colorize diagnostics: auto (default), always or never
  -V, --version          Print the compiler version
  -h, --help             Print this help";

struct Options {
    input: PathBuf,
    output: Option<PathBuf>,
    color: ColorChoice,
    check_only: bool,
    nostdlib: bool,
    target: Option<String>,
}

fn parse_color(value: &str) -> Option<ColorChoice> {
    match value {
        "auto" => Some(ColorChoice::Auto),
        "always" => Some(ColorChoice::Always),
        "never" => Some(ColorChoice::Never),
        _ => None,
    }
}

/// Prints a diagnostic that is not tied to any source file and exits.
fn fatal(color: ColorChoice, d: Diagnostic, code: i32) -> ! {
    let mut em = Emitter::new(color);
    em.emit(&d, &SourceMap::new());
    process::exit(code);
}

fn parse_args() -> Options {
    let mut color = ColorChoice::Auto;
    let mut check_only = false;
    let mut nostdlib = false;
    let mut target: Option<String> = None;
    let mut positional: Vec<String> = Vec::new();

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--version" | "-V" => {
                println!("Marrow version: v{}", env!("CARGO_PKG_VERSION"));
                process::exit(0);
            }
            "--help" | "-h" => {
                println!("{}", USAGE);
                process::exit(0);
            }
            "--check" => check_only = true,
            "--nostdlib" => nostdlib = true,
            "--target" => match args.next() {
                Some(v) => target = Some(v),
                None => fatal(color, Diagnostic::error("option `--target` requires a value").help("example: `--target aarch64-linux`"), 2),
            },
            "--color" => match args.next() {
                Some(v) => match parse_color(&v) {
                    Some(c) => color = c,
                    None => fatal(
                        color,
                        Diagnostic::error(format!("invalid value `{}` for `--color`", v)).help("possible values are `auto`, `always` and `never`"),
                        2,
                    ),
                },
                None => fatal(color, Diagnostic::error("option `--color` requires a value").help("possible values are `auto`, `always` and `never`"), 2),
            },
            other if other.starts_with("--color=") => match parse_color(&other["--color=".len()..]) {
                Some(c) => color = c,
                None => fatal(
                    color,
                    Diagnostic::error(format!("invalid value `{}` for `--color`", &other["--color=".len()..]))
                        .help("possible values are `auto`, `always` and `never`"),
                    2,
                ),
            },
            other if other.starts_with('-') && other.len() > 1 => {
                fatal(color, Diagnostic::error(format!("unknown option `{}`", other)).help("run `marrow --help` to see the available options"), 2)
            }
            _ => positional.push(arg),
        }
    }

    if positional.is_empty() {
        eprintln!("{}", USAGE);
        process::exit(2);
    }
    if positional.len() > 2 {
        fatal(
            color,
            Diagnostic::error(format!("unexpected argument `{}`", positional[2])).help("usage: marrow [OPTIONS] <input.mw> [output.ssa]"),
            2,
        );
    }

    let mut it = positional.into_iter();
    Options {
        input: PathBuf::from(it.next().unwrap()),
        output: it.next().map(PathBuf::from),
        color,
        check_only,
        nostdlib,
        target,
    }
}

/// `marrow --dump-tokens <file>`: prints one line per token (`Kind start end [payload]`,
/// offsets in characters). Used to check the self-hosted lexer against this one.
fn dump_tokens(path: &str) -> ! {
    let src = std::fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("cannot read {}: {}", path, e);
        process::exit(2)
    });
    let chars: Vec<char> = src.chars().collect();
    let esc = |bytes: &[u8]| -> String {
        let mut o = String::new();
        for &b in bytes {
            if (32..127).contains(&b) && b != b'\\' && b != b'"' {
                o.push(b as char);
            } else {
                o.push_str(&format!("\\x{:02x}", b));
            }
        }
        o
    };
    match lexer::Lexer::new(&src, 0).tokenize() {
        Ok(tokens) => {
            for t in tokens {
                let dbg = format!("{:?}", t.kind);
                let name = dbg.split('(').next().unwrap_or("").to_string();
                let payload = match &t.kind {
                    token::TokenKind::Identifier(n) => format!(" {}", esc(n.as_bytes())),
                    token::TokenKind::IntLiteral(v) => format!(" {}", v),
                    token::TokenKind::FloatLiteral(_) => format!(" {}", chars[t.span.start..t.span.end].iter().collect::<String>()),
                    token::TokenKind::StringLiteral(s) => format!(" {}", esc(s.as_bytes())),
                    token::TokenKind::CharLiteral(c) => format!(" {}", *c as u32),
                    _ => String::new(),
                };
                println!("{} {} {}{}", name, t.span.start, t.span.end, payload);
            }
            process::exit(0);
        }
        Err(d) => {
            let l = d.labels.iter().find(|l| l.primary);
            let (a, b) = l.map_or((0, 0), |l| (l.span.start, l.span.end));
            println!("Error {} {} {}", d.code.unwrap_or("?"), a, b);
            process::exit(1);
        }
    }
}

/// `marrow --dump-ast <file>`: parses one file (imports are not followed) and prints the AST
/// in the format of `src/dump.rs`, or `Error <code> <start> <end>`.
fn dump_ast(path: &str) -> ! {
    let src = std::fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("cannot read {}: {}", path, e);
        process::exit(2)
    });
    let file = diagnostic::SourceFile::new(path.to_string(), &src, false);
    let fail = |d: Diagnostic| -> ! {
        let l = d.labels.iter().find(|l| l.primary);
        let (a, b) = l.map_or((0, 0), |l| (l.span.start, l.span.end));
        println!("Error {} {} {}", d.code.unwrap_or("?"), a, b);
        process::exit(1)
    };
    let tokens = lexer::Lexer::new(&src, 0).tokenize().unwrap_or_else(|d| fail(d));
    match parser::Parser::new(tokens, &file).parse_program() {
        Ok(p) => {
            print!("{}", dump::dump_program(&p));
            process::exit(0)
        }
        Err(d) => fail(d),
    }
}

/// `marrow --dump-imports <file> <target>`: loads a file with its imports and prints the merged item
/// order (`Item <canonical path> <start> <end>` then `Library <bool>`) or every diagnostic
/// (`Error <code> <canonical path or -> <start> <end>`).
fn dump_imports(path: &str, target: &str) -> ! {
    let mut sm = diagnostic::SourceMap::new();
    let mut diags: Vec<Diagnostic> = Vec::new();
    let canon = |name: &str| -> String {
        std::fs::canonicalize(name).map(|p| p.display().to_string()).unwrap_or_else(|_| name.to_string())
    };
    let loaded = import::load_with_imports(std::path::Path::new(path), target, &mut sm, &mut diags);
    let mut failed = false;
    for d in &diags {
        if !d.is_error() {
            continue;
        }
        failed = true;
        let l = d.labels.iter().find(|l| l.primary);
        match l {
            Some(l) => println!("Error {} {} {} {}", d.code.unwrap_or("?"), canon(&sm.file(l.span.file).name), l.span.start, l.span.end),
            None => println!("Error {} - 0 0", d.code.unwrap_or("?")),
        }
    }
    if let (Some((program, lib)), false) = (loaded, failed) {
        for it in &program.items {
            println!("Item {} {} {}", canon(&sm.file(it.span.file).name), it.span.start, it.span.end);
        }
        println!("Library {}", lib);
        process::exit(0);
    }
    process::exit(1);
}

/// `marrow --dump-lowered <file> <target>`: loads a file with its imports, lowers it and prints
/// every diagnostic of the lowering (`Error|Warning <code> <file> <start> <end> <message>`), then,
/// when there is no error, each item as `@<canonical path>` followed by its S-expression.
fn dump_lowered(path: &str, target: &str) -> ! {
    let mut sm = diagnostic::SourceMap::new();
    let mut diags: Vec<Diagnostic> = Vec::new();
    let canon = |name: &str| -> String {
        std::fs::canonicalize(name).map(|p| p.display().to_string()).unwrap_or_else(|_| name.to_string())
    };
    let line = |sm: &diagnostic::SourceMap, d: &Diagnostic, msg: bool| -> String {
        let m = if msg { format!(" {}", d.message) } else { String::new() };
        let l = d.labels.iter().find(|l| l.primary);
        let level = if d.is_error() { "Error" } else { "Warning" };
        match l {
            Some(l) => format!("{} {} {} {} {}{}", level, d.code.unwrap_or("?"), canon(&sm.file(l.span.file).name), l.span.start, l.span.end, m),
            None => format!("{} {} - 0 0{}", level, d.code.unwrap_or("?"), m),
        }
    };
    let loaded = import::load_with_imports(std::path::Path::new(path), target, &mut sm, &mut diags);
    let mut failed = false;
    for d in &diags {
        if d.is_error() {
            failed = true;
            println!("{}", line(&sm, d, false));
        }
    }
    let Some((program, _)) = loaded else { process::exit(1) };
    if failed {
        process::exit(1);
    }
    let (lowered, ds) = lower::lower(program);
    let mut bad = false;
    for d in &ds {
        bad |= d.is_error();
        println!("{}", line(&sm, d, true));
    }
    if bad {
        process::exit(1);
    }
    for it in &lowered.items {
        println!("@{}", canon(&sm.file(it.span.file).name));
        println!("{}", dump::dump_item(it));
    }
    process::exit(0);
}

fn main() {
    {
        let argv: Vec<String> = env::args().collect();
        if let Some(i) = argv.iter().position(|a| a == "--dump-lowered") {
            match (argv.get(i + 1), argv.get(i + 2)) {
                (Some(p), Some(t)) => dump_lowered(p, t),
                _ => {
                    eprintln!("--dump-lowered needs a file and a target");
                    process::exit(2)
                }
            }
        }
    }
    {
        let argv: Vec<String> = env::args().collect();
        if let Some(i) = argv.iter().position(|a| a == "--dump-imports") {
            match (argv.get(i + 1), argv.get(i + 2)) {
                (Some(p), Some(t)) => dump_imports(p, t),
                _ => {
                    eprintln!("--dump-imports needs a file and a target");
                    process::exit(2)
                }
            }
        }
        if let Some(i) = argv.iter().position(|a| a == "--dump-ast") {
            match argv.get(i + 1) {
                Some(p) => dump_ast(p),
                None => {
                    eprintln!("--dump-ast needs a file");
                    process::exit(2)
                }
            }
        }
        if let Some(i) = argv.iter().position(|a| a == "--dump-tokens") {
            match argv.get(i + 1) {
                Some(p) => dump_tokens(p),
                None => {
                    eprintln!("--dump-tokens needs a file");
                    process::exit(2)
                }
            }
        }
    }
    let opts = parse_args();
    let input_path = opts.input.clone();

    if !input_path.exists() {
        fatal(
            opts.color,
            Diagnostic::error(format!("input file `{}` not found", input_path.display())),
            2,
        );
    }

    let output_path = opts.output.clone().unwrap_or_else(|| {
        let mut path = input_path.clone();
        path.set_extension("ssa");
        path
    });

    let target = match &opts.target {
        Some(t) => Target::parse(t),
        None => Target::host(),
    }
    .unwrap_or_else(|d| fatal(opts.color, d, 2));

    let mut em = Emitter::new(opts.color);
    let mut sm = SourceMap::new();
    let mut diags: Vec<Diagnostic> = Vec::new();

    let loaded = import::load_with_imports(&input_path, &target.dir_name(), &mut sm, &mut diags);
    em.emit_all(&diags, &sm);
    let (program, is_library) = match loaded {
        Some(loaded) if !em.has_errors() => loaded,
        _ => {
            em.finish();
            process::exit(1);
        }
    };

    let (program, lower_diags) = lower::lower(program);
    em.emit_all(&lower_diags, &sm);
    if em.has_errors() {
        em.finish();
        process::exit(1);
    }

    let type_diags = typeck::check(&program, &sm);
    em.emit_all(&type_diags, &sm);
    em.finish();
    if em.has_errors() {
        process::exit(1);
    }

    if opts.check_only {
        eprintln!("Check finished: no errors found.");
        return;
    }

    let qbe_il = match codegen::generate(&program) {
        Ok(il) => il,
        Err(d) => {
            em.emit(&d, &sm);
            process::exit(1);
        }
    };

    fs::write(&output_path, &qbe_il).unwrap_or_else(|err| {
        fatal(opts.color, Diagnostic::error(format!("failed to write `{}`: {}", output_path.display(), err)), 2);
    });

    let qbe_output_path = output_path.with_extension("s");
    let mut qbe_cmd = std::process::Command::new("qbe");
    if opts.target.is_some() {
        qbe_cmd.arg("-t").arg(target.qbe_target());
    }
    let qbe_status = qbe_cmd
        .arg(&output_path)
        .arg("-o")
        .arg(&qbe_output_path)
        .status()
        .unwrap_or_else(|err| {
            fatal(
                opts.color,
                Diagnostic::error(format!("failed to execute `qbe`: {}", err)).help("is QBE installed and available in your PATH? see https://c9x.me/compile/"),
                2,
            );
        });

    if !qbe_status.success() {
        fatal(opts.color, Diagnostic::error(format!("`qbe` failed with exit code {}", qbe_status.code().unwrap_or(-1))), 1);
    }

    // QBE picks its backend from the host unless told otherwise.
    let cross = opts.target.is_some() && !target.is_host();
    if opts.nostdlib {
        if let Err(d) = target.check_nostdlib_supported() {
            fatal(opts.color, d, 2);
        }
    }
    {
        let mut asm = fs::read_to_string(&qbe_output_path).unwrap_or_default();
        asm = target.adapt_asm(&asm);
        asm.push_str(&target.runtime_asm(opts.nostdlib));
        if let Err(err) = fs::write(&qbe_output_path, asm) {
            fatal(opts.color, Diagnostic::error(format!("failed to write `{}`: {}", qbe_output_path.display(), err)), 2);
        }
    }
    if cross {
        eprintln!("Cross target {}: assembly written to {}", target.dir_name(), qbe_output_path.display());
        return;
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
                fatal(opts.color, Diagnostic::error(format!("failed to execute `{}`: {}", compiler, err)), 2);
            });

        if !compiler_status.success() {
            fatal(
                opts.color,
                Diagnostic::error(format!("`{}` failed with exit code {}", compiler, compiler_status.code().unwrap_or(-1))),
                1,
            );
        }

        eprintln!(
            "Library compiled (no `main`, see `@no_main`): {}\nLink it into a program with: cc your_program.o {} -o your_program",
            object_output_path.display(),
            object_output_path.display()
        );
        return;
    }

    let executable_output_path = input_path.with_extension("");
    let mut link = std::process::Command::new(compiler);
    if opts.nostdlib && target.freestanding_link() {
        link.args(["-nostdlib", "-static"]);
    }
    if cfg!(target_os = "windows") {
        // Winsock backs the socket calls of the Windows platform layer.
        link.arg("-lws2_32");
    }
    let compiler_status = link
        .arg(&qbe_output_path)
        .arg("-o")
        .arg(&executable_output_path)
        .status()
        .unwrap_or_else(|err| {
            fatal(opts.color, Diagnostic::error(format!("failed to execute `{}`: {}", compiler, err)), 2);
        });

    if !compiler_status.success() {
        fatal(
            opts.color,
            Diagnostic::error(format!("`{}` failed with exit code {}", compiler, compiler_status.code().unwrap_or(-1))),
            1,
        );
    }

    eprintln!("Compilation successful! Executable created at: {}", executable_output_path.display());
}
