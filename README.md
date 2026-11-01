<p align="center">
  <img src="icon.png" width="128" height="128" alt="Marrow logo">
</p>

<h1 align="center">Marrow Programming Language</h1>

<p align="center">
  Syntax highlighting, diagnostics and formatting for <b>Marrow</b> in Visual Studio Code.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/VS_Code-%5E1.125.0-007ACC?style=for-the-badge&logo=visualstudiocode" alt="VS Code version">
  <img src="https://img.shields.io/badge/Marrow-v0.4.0%2B-8A2BE2?style=for-the-badge" alt="Marrow version">
  <img src="https://img.shields.io/badge/License-MIT-green?style=for-the-badge" alt="License">
</p>

---

## Features

- **Syntax highlighting** for `.mw` files, from a TextMate grammar: keywords (`fn`, `struct`, `enum`, `var`, `const`, `match`, `unsafe`, `raw`, `move`, `cast`, `sizeof`…), primitive types, decorators (`@export`, `@trusted`, `@unsafe`, `@drop(...)`…), strings, numbers and comments.
- **Diagnostics as you work.** The extension starts the language server of the compiler (`marrow lsp`): the errors and warnings of `marrow check` (type errors, ownership and borrow errors, `unsafe` violations, lints) appear in the editor and in the Problems panel.
- **Format Document** runs `marrow fmt`, which keeps your comments and refuses to change anything but layout.
- **Editing helpers:** line comments (`//`) and block comments (`/* */`), bracket and quote auto-closing, surrounding pairs.

## Requirements

The language features need the `marrow` binary, **v0.4.0 or later**, in your `PATH`. Syntax highlighting works without it. If `marrow` lives elsewhere, set `marrow.path`.

## Settings

| Setting | Default | Description |
| --- | --- | --- |
| `marrow.path` | `marrow` | Path of the `marrow` binary used to run `marrow lsp`. |

If the server cannot start, a warning names the command that failed.

## Code preview

```marrow
@import("std/std.mw")

struct Point {
    x: f64;
    y: f64;
}

fn norm2(p: &Point) -> f64 {
    ret p.x * p.x + p.y * p.y;
}

@export fn main(argc: i32, argv: rawptr) -> i32 {
    var items: ^VecPtr = vec_ptr_new(4);   // owned: freed at the end of the scope
    var p: Point = Point { x: 3.0, y: 4.0 };
    if norm2(&p) > 20.0 { vec_ptr_push(items, cast(rawptr) 1); }
    println_i64(vec_ptr_len(items));
    ret 0;
}
```

## Installation

From a `.vsix` file:

```bash
code --install-extension marrow-vscode-0.1.0.vsix
```

From the sources (this branch), to try or to package the extension:

```bash
npm install                # installs vscode-languageclient
npx @vscode/vsce package   # builds marrow-vscode-<version>.vsix
```

To debug it, open the folder in VS Code and press `F5`: a new window starts with the extension loaded (see `.vscode/launch.json`). **Developer: Inspect Editor Tokens and Scopes** shows how the grammar tokenizes a file.

## Links

- Compiler, documentation and language reference: <https://github.com/zuygui/marrow>
- Report a problem with the extension: <https://github.com/zuygui/marrow/issues>
