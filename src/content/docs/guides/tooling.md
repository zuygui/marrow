---
title: Formatter, docs and editor support
description: "marrow fmt, marrow doc and marrow lsp: format code, generate documentation and get diagnostics in your editor."
sidebar:
  order: 5
---

Three commands of the compiler help you write code: a formatter, a documentation generator and a language server. None of them needs `qbe` or a C compiler.

## `marrow fmt`

```sh
marrow fmt                 # format the project (or the current folder)
marrow fmt src/main.mw     # format one file
marrow fmt --check         # change nothing, exit with 1 if a file would change
```

`marrow fmt [--check] [PATH...]` formats every `.mw` file under the given files and folders. Hidden folders, `build/`, `vendor/` and `target/` are skipped. With no path, the project is formatted.

What it normalizes:

- **Indentation**: 4 spaces per `{`, and one more for a line that continues an expression (inside `(`/`[`, or after or before a binary operator).
- **Spaces** between tokens: `a+b` becomes `a + b`, `f( x )` becomes `f(x)`, `x :i64` becomes `x: i64`.
- **Blank lines**: at most one in a row, none right after `{` or before `}`.
- **The end of the file**: trailing spaces are removed, and there is exactly one newline.

Comments are kept where they are, and so is the way you split your code into lines. A comment at the end of a line keeps the spaces in front of it, so aligned comments stay aligned.

The formatter works on the tokens, not on the syntax tree, and it checks its own work: after formatting a file it compares the tokens of the result with the ones of the input, and refuses to write the file if they differ. A file that cannot be read as tokens (an unterminated string, for example) is reported and left alone.

| Output | Meaning |
|---|---|
| `Formatted <file>` | The file was rewritten. |
| `Would reformat: <file>` | With `--check`: the file is not formatted. |

The exit code is `1` when `--check` finds a file to change, or when a file could not be formatted.

Use `marrow fmt --check` in your CI to keep a project formatted.

## `marrow doc`

```sh
marrow doc                 # document src/, write the pages in build/doc/
marrow doc -o docs lib/    # another folder, another source
```

`marrow doc [-o DIR] [PATH...]` writes Markdown pages from the comments of your code: one page per source file, plus an `index.md`.

A run of `///` lines right before a top-level `fn`, `struct`, `enum` or `var` documents it. `//!` lines at the top of a file describe the file.

```marrow
//! 2D geometry helpers.

/// A point on the plane.
struct Point {
    x: i64;
    y: i64;
}

/// The squared distance between two points.
/// Cheaper than a distance, and enough to compare two of them.
fn dist2(a: Point, b: Point) -> i64 {
    var dx: i64 = a.x - b.x;
    var dy: i64 = a.y - b.y;
    ret dx * dx + dy * dy;
}
```

For each item the page shows its declaration (a function without its body) and then your text. A blank `///` line starts a new paragraph, and `` `code` `` is kept as written.

Left out of the pages: names that start with `_`, and `@test` functions. Items without a `///` comment are listed with only their declaration.

The default source is the project's `src/` folder. The pages are named after the path of the file, with `/` written as `.`: `src/lib.mw` gives `src.lib.md`.

## `marrow lsp`

`marrow lsp` is a language server: it talks the [Language Server Protocol](https://microsoft.github.io/language-server-protocol/) on its standard input and output, so any editor with an LSP client can use it. Configure the editor to start `marrow lsp` for `.mw` files.

| Feature | How it works |
|---|---|
| Diagnostics | The errors and warnings of `marrow check`, for the whole project, sent when a file is opened or saved. They follow your [lint](/marrow/guides/lints/) levels. |
| Formatting | `textDocument/formatting` uses the same formatter as `marrow fmt`, on the text in the editor, saved or not. |

The compiler reads files from disk, so a change is checked when the file is saved. Hover, go to definition and symbols are not implemented yet.

### Diagnostics as JSON

The server is built on a general option of the compiler, which you can use in your own tools:

```sh
marrow check --message-format json src/main.mw
```

Every diagnostic is printed on its own line of standard error as a JSON object, and nothing else is printed:

```json
{"code":"E0402","severity":"error","message":"mismatched types","file":"/home/me/app/src/main.mw","line":3,"col":16,"endLine":3,"endCol":19}
```

`line` and `col` are 1-based. `severity` is `error` or `warning`; a lint set to `deny` is an `error`. The default is `--message-format human`.
