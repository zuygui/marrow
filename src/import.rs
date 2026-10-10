use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{Decorator, ExprKind, GlobalItem, Program};
use crate::diagnostic::{Diagnostic, SourceMap, Span};
use crate::lexer::Lexer;
use crate::parser::Parser;

fn get_marrow_home() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

fn marrow_dir() -> Option<PathBuf> {
    get_marrow_home().map(|h| h.join(".marrow"))
}

/// Loads `entry_path` and everything it imports. Diagnostics are appended to
/// `diags`; `None` is returned when at least one file could not be loaded.
pub fn load_with_imports(entry_path: &Path, target: &str, sm: &mut SourceMap, diags: &mut Vec<Diagnostic>) -> Option<(Program, bool)> {
    let mut visited = HashSet::new();
    let mut items = Vec::new();
    let entry_dir = entry_path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    let errors_before = diags.iter().filter(|d| d.is_error()).count();
    let entry_items = load_file(entry_path, &entry_dir, target, &mut visited, &mut items, sm, diags, None)?;
    if diags.iter().filter(|d| d.is_error()).count() > errors_before {
        return None;
    }
    let is_library = entry_items.iter().any(|it| find_decorator(&it.decorators, "no_main").is_some());
    Some((Program { items }, is_library))
}

fn load_file(
    path: &Path,
    entry_dir: &Path,
    target: &str,
    visited: &mut HashSet<PathBuf>,
    items: &mut Vec<GlobalItem>,
    sm: &mut SourceMap,
    diags: &mut Vec<Diagnostic>,
    imported_at: Option<Span>,
) -> Option<Vec<GlobalItem>> {
    let filename = path.display().to_string();
    let canon = match fs::canonicalize(path) {
        Ok(c) => c,
        Err(e) => {
            diags.push(cannot_open(&filename, &e.to_string(), imported_at));
            return None;
        }
    };
    if !visited.insert(canon) {
        return Some(Vec::new());
    }

    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            diags.push(cannot_open(&filename, &e.to_string(), imported_at));
            return None;
        }
    };

    let external = marrow_dir().map(|d| path.starts_with(&d)).unwrap_or(false);
    let file_id = sm.add_file(filename.clone(), &source, external);

    let tokens = match Lexer::new(&source, file_id).tokenize() {
        Ok(t) => t,
        Err(d) => {
            diags.push(d);
            return None;
        }
    };
    let program = match Parser::new(tokens, sm.file(file_id)).parse_program() {
        Ok(p) => p,
        Err(d) => {
            diags.push(d);
            return None;
        }
    };

    let base_dir = path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));

    let mut own_items = Vec::with_capacity(program.items.len());
    for item in program.items {
        for (import, span) in import_targets(&item.decorators, diags) {
            // `{target}` expands to e.g. `linux_x86_64` (see runtime.rs).
            let import = import.replace("{target}", target);
            match resolve_import(&import, &base_dir, entry_dir) {
                Ok(resolved) => {
                    load_file(&resolved, entry_dir, target, visited, items, sm, diags, Some(span));
                }
                Err(tried) => {
                    let list = tried.iter().map(|p| format!("  - {}", p.display())).collect::<Vec<_>>().join("\n");
                    diags.push(
                        Diagnostic::error(format!("unresolved import `{}`", import))
                            .code("E0001")
                            .primary(span, "no such file or library")
                            .note(format!("looked in these locations:\n{}", list))
                            .help("paths are relative to the importing file, to the entry file, or to the `~/.marrow` directory"),
                    );
                }
            }
        }
        items.push(item.clone());
        own_items.push(item);
    }

    Some(own_items)
}

fn cannot_open(filename: &str, reason: &str, imported_at: Option<Span>) -> Diagnostic {
    let d = Diagnostic::error(format!("couldn't read `{}`: {}", filename, reason)).code("E0004");
    match imported_at {
        Some(span) => d.primary(span, "imported here"),
        None => d,
    }
}

fn find_decorator<'a>(decorators: &'a [Decorator], name: &str) -> Option<&'a Decorator> {
    decorators.iter().find(|d| d.name == name)
}

fn import_targets(decorators: &[Decorator], diags: &mut Vec<Diagnostic>) -> Vec<(String, Span)> {
    let mut out = Vec::new();
    for d in decorators.iter().filter(|d| d.name == "import") {
        let args = d.args.clone().unwrap_or_default();
        if args.len() != 1 {
            diags.push(
                Diagnostic::error(format!("`@import` takes exactly one argument, but {} were supplied", args.len()))
                    .code("E0002")
                    .primary(d.span, "expected one argument: the path of the file or library to import")
                    .help("write it like this: `@import(\"std\")`"),
            );
            continue;
        }
        match &args[0].kind {
            ExprKind::StringLiteral(s) => out.push((s.clone(), d.span)),
            _ => {
                diags.push(
                    Diagnostic::error("`@import` expects a string literal")
                        .code("E0003")
                        .primary(args[0].span, "expected a string literal")
                        .help("write it like this: `@import(\"std\")`"),
                );
            }
        }
    }
    out
}

fn resolve_import(target: &str, carrier_dir: &Path, entry_dir: &Path) -> Result<PathBuf, Vec<PathBuf>> {
    let mut tried = Vec::new();

    let mut bases = vec![carrier_dir.to_path_buf()];

    if entry_dir != carrier_dir {
        bases.push(entry_dir.to_path_buf());
    }

    if let Some(marrow_dir) = marrow_dir() {
        let marrow_std = marrow_dir.join("std");
        if marrow_std.exists() {
            bases.push(marrow_std);
        }

        if marrow_dir.exists() {
            bases.push(marrow_dir);
        }
    }

    for base in bases {
        let direct = base.join(target);

        if direct.is_file() {
            return Ok(direct);
        }
        tried.push(direct.clone());

        if !target.ends_with(".mw") && !target.ends_with(".mrw") {
            let with_mw = base.join(format!("{}.mw", target));
            if with_mw.is_file() {
                return Ok(with_mw);
            }
            tried.push(with_mw);

            let with_mrw = base.join(format!("{}.mrw", target));
            if with_mrw.is_file() {
                return Ok(with_mrw);
            }
            tried.push(with_mrw);
        }

        if direct.is_dir() {
            if let Some(dir_name) = direct.file_name().and_then(|n| n.to_str()) {
                let umbrella_mw = direct.join(format!("{}.mw", dir_name));
                if umbrella_mw.is_file() {
                    return Ok(umbrella_mw);
                }
                tried.push(umbrella_mw);

                let umbrella_mrw = direct.join(format!("{}.mrw", dir_name));
                if umbrella_mrw.is_file() {
                    return Ok(umbrella_mrw);
                }
                tried.push(umbrella_mrw);
            }
        }
    }

    Err(tried)
}
