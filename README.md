<p align="center">
  <img src="icon.png" width="128" height="128" alt="Marrow Logo">
</p>

<h1 align="center">Marrow Language Support</h1>

<p align="center">
  Official Visual Studio Code extension for the <b>Marrow</b> programming language.
</p>

<p align="center">
  <a href="https://marketplace.visualstudio.com/items?itemName=marrow-lang.marrow-vscode">
    <img src="https://img.shields.io/badge/Marrow-v0.1.0-8A2BE2?style=for-the-badge&logo=visualstudiocode" alt="Marrow Version">
  </a>
  <a href="https://marketplace.visualstudio.com/items?itemName=marrow-lang.marrow-vscode">
    <img src="https://img.shields.io/badge/VS_Code-^1.75.0-007ACC?style=for-the-badge&logo=visualstudiocode" alt="VS Code Version">
  </a>
  <a href="https://github.com/marrow-lang/marrow">
    <img src="https://img.shields.io/badge/License-MIT-green?style=for-the-badge" alt="License">
  </a>
</p>

---

## ✨ Features

- **🎨 Rich Syntax Highlighting:** Powered by a custom TextMate grammar covering keywords (`fn`, `struct`, `ret`, `cast`), types (`i64`, `f64`, `rawptr`), decorators (`@`), strings, numbers, and comments.
- **📄 Dual Extension Support:** Native support for both `.mrw` and `.marrow` files.
- **💬 Smart Commenting:** Toggle line comments (`//`) and block comments (`/* */`) using native shortcuts.
- **🔒 Auto-Closing Pairs:** Automatic pair completion for `{ }`, `[ ]`, `( )`, `" "`, and `' '`.

---

## 💻 Code Preview

```marrow
@inline
fn factorial(n: i64) -> i64 {
    // Calculate factorial recursively
    if (n <= 1) {
        ret 1;
    }
    ret n * factorial(n - 1);
}

struct Point {
    x: f64,
    y: f64,
}

fn main() -> i32 {
    const p = Point { x: 10.5, y: 20.0 };
    ret 0;
}
```

## ⌨️ Keyboard Shortcuts

Feature | Windows / Linux | macOS
--- | --- | --- 
Toggle Line Comment | |Ctrl + /` | `Cmd + /`
Toggle Block Comment | `Shift + Alt + A` | `Shift + Option + A`
Inspect Tokens & Scopes | `Ctrl + Shift + P` → Inspect Editor Tokens | `Cmd + Shift + P` → Inspect Editor Tokens

## 🚀 Installation

### From the VS Code Marketplace
1. Open VS Code.
2. Press `Ctrl + Shift + X` (or  `Cmd + Shift + X` on macOS).
3. Search for Marrow Language Support.
4. Click Install.

### Manual Installation (.vsix)
If you built the extension locally using vsce package:

```bash
code --install-extension marrow-vscode-0.1.0.vsix
```