use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{Decorator, ExprKind, GlobalItem, Program};
use crate::error::CompileError;
use crate::lexer::Lexer;
use crate::parser::Parser;

pub fn load_with_imports(entry_path: &Path) -> Result<(Program, bool), String> {
    let mut visited = HashSet::new();
    let mut items = Vec::new();
    let entry_dir = entry_path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    let entry_items = load_file(entry_path, &entry_dir, &mut visited, &mut items)?;
    let is_library = entry_items.iter().any(|it| find_decorator(&it.decorators, "no_main").is_some());
    Ok((Program { items }, is_library))
}

fn load_file(path: &Path, entry_dir: &Path, visited: &mut HashSet<PathBuf>, items: &mut Vec<GlobalItem>) -> Result<Vec<GlobalItem>, String> {
    let canon = fs::canonicalize(path).map_err(|e| format!("impossible d'ouvrir '{}': {}", path.display(), e))?;
    if !visited.insert(canon) {
        return Ok(Vec::new());
    }

    let filename = path.display().to_string();
    let source = fs::read_to_string(path).map_err(|e| format!("impossible de lire '{}': {}", filename, e))?;

    let tokens = Lexer::new(&source).tokenize().map_err(|e| e.render(&filename, &source))?;
    let program = Parser::new(tokens).parse_program().map_err(|e| e.render(&filename, &source))?;

    let base_dir = path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));

    let mut own_items = Vec::with_capacity(program.items.len());
    for item in program.items {
        for (target, line, col) in import_targets(&item.decorators, &filename, &source)? {
            let resolved = resolve_import(&target, &base_dir, entry_dir).map_err(|tried| {
                let list = tried.iter().map(|p| format!("  - {}", p.display())).collect::<Vec<_>>().join("\n");
                CompileError::new(
                    line,
                    col,
                    1,
                    format!("import introuvable pour '@import(\"{}\")'\nchemins essayés :\n{}", target, list),
                )
                .render(&filename, &source)
            })?;
            load_file(&resolved, entry_dir, visited, items)?;
        }
        items.push(item.clone());
        own_items.push(item);
    }

    Ok(own_items)
}

fn find_decorator<'a>(decorators: &'a [Decorator], name: &str) -> Option<&'a Decorator> {
    decorators.iter().find(|d| d.name == name)
}

fn import_targets(decorators: &[Decorator], filename: &str, source: &str) -> Result<Vec<(String, usize, usize)>, String> {
    let mut out = Vec::new();
    for d in decorators.iter().filter(|d| d.name == "import") {
        let args = d.args.clone().unwrap_or_default();
        if args.len() != 1 {
            return Err(CompileError::new(
                d.line,
                d.col,
                1,
                "'@import' attend exactement un argument : le chemin du fichier (ou de la bibliothèque) à importer, ex. '@import(\"std\")'",
            )
            .render(filename, source));
        }
        match &args[0].kind {
            ExprKind::StringLiteral(s) => out.push((s.clone(), d.line, d.col)),
            _ => {
                return Err(CompileError::new(d.line, d.col, 1, "'@import' : le chemin doit être une chaîne littérale").render(filename, source));
            }
        }
    }
    Ok(out)
}

fn resolve_import(target: &str, carrier_dir: &Path, entry_dir: &Path) -> Result<PathBuf, Vec<PathBuf>> {
    let mut tried = Vec::new();

    let mut bases = vec![carrier_dir.to_path_buf()];
    if entry_dir != carrier_dir {
        bases.push(entry_dir.to_path_buf());
    }

    for base in bases {
        let direct = base.join(target);
        if direct.is_file() {
            return Ok(direct);
        }
        tried.push(direct.clone());

        if !target.ends_with(".mrw") {
            let with_ext = base.join(format!("{}.mrw", target));
            if with_ext.is_file() {
                return Ok(with_ext);
            }
            tried.push(with_ext);
        }

        if direct.is_dir() {
            if let Some(dir_name) = direct.file_name().and_then(|n| n.to_str()) {
                let umbrella = direct.join(format!("{}.mrw", dir_name));
                if umbrella.is_file() {
                    return Ok(umbrella);
                }
                tried.push(umbrella);
            }
        }
    }

    Err(tried)
}