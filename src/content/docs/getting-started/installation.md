---
title: Installation
description: How to install the marrow compiler and the QBE backend it depends on.
sidebar:
  order: 2
---

Marrow ships as a single `marrow` binary plus the [QBE](https://c9x.me/compile/) backend binary and the `std/` sources. All three are bundled together in each GitHub release.

## Requirements

Whatever installation method you use, you will also need **a system C compiler** available on your `PATH`:

- **Linux / macOS:** `cc` (or any compiler that responds to the `cc` alias, e.g. GCC or Clang).
- **Windows:** `gcc`.

This is required because Marrow does not link executables itself — it generates QBE IL, asks the `qbe` binary to turn that into assembly, and then shells out to your C compiler to assemble and link the final binary. If no supported C compiler is found for your OS, the compiler will stop after producing the `.s` assembly file only.

## Linux & macOS

Run the install script. It downloads the right release archive for your CPU architecture, unpacks the `marrow` and `qbe` binaries plus the `std/` library into `~/.marrow`, and appends `~/.marrow/bin` to your shell profile's `PATH`.

```bash
curl -fsSL https://raw.githubusercontent.com/zuygui/marrow/main/install.sh | sh
```

What it does, concretely:

1. Detects your OS (`Linux` / `Darwin`) and architecture (`x86_64` / `aarch64`/`arm64`).
2. Downloads `https://github.com/zuygui/marrow/releases/latest/download/marrow-<arch>-<platform>.tar.gz`.
3. Installs the binaries to `~/.marrow/bin/marrow` and `~/.marrow/bin/qbe`.
4. Copies the bundled `std/` sources to `~/.marrow/std` (this is what makes `@import("std/...")` resolve out of the box — see [Modules & imports](/language/decorators-and-modules/)).
5. Adds `export PATH="$HOME/.marrow/bin:$PATH"` to `~/.zshrc`, `~/.bashrc`, or `~/.profile` (whichever matches your `$SHELL`).

After it finishes, either restart your terminal or re-source your profile, then check that everything is on `PATH`:

```bash
marrow --version
qbe -h
```

### Supported targets

The official release archives currently cover:

| OS      | Architecture      |
|---------|--------------------|
| Linux   | `x86_64`           |
| macOS   | `aarch64` (Apple Silicon) |

If your architecture isn't published as a release yet, you can always build from source (see below).

## Windows

There is a Windows release archive (`marrow-x86_64-pc-windows-msvc.zip`) published alongside every release, containing `marrow.exe`, `qbe.exe` and `std/`. For now, install it manually:

1. Download `marrow-x86_64-pc-windows-msvc.zip` from the [latest release](https://github.com/zuygui/marrow/releases/latest).
2. Extract it somewhere, e.g. `%USERPROFILE%\.marrow`.
3. Add that folder to your `PATH` so `marrow.exe` and `qbe.exe` are reachable.
4. Make sure `gcc` is installed and on `PATH` (e.g. via MSYS2/MinGW-w64) — it's what Marrow shells out to for the final assemble + link step on Windows.

:::note
An `install.ps1` script exists in the repository for Windows, but at the time of writing it mirrors `install.sh` line-for-line rather than being genuine PowerShell — treat the manual steps above as the reliable path on Windows until that script is fixed upstream.
:::

## Building from source

Marrow's own compiler is a small Rust (Cargo) project with **no external crate dependencies**, so building it is a plain `cargo build`:

```bash
git clone https://github.com/zuygui/marrow.git
cd marrow
cargo build --release
# binary is at target/release/marrow
```

You'll also need to build QBE itself, since it's a separate C project:

```bash
git clone git://c9x.me/qbe.git
cd qbe
make
# binary is at ./qbe — put it on your PATH, alongside marrow
```

Finally, make sure the `std/` folder from this repository is discoverable — either copy it to `~/.marrow/std`, or keep it next to the source files you're compiling (see [Modules & imports](/language/decorators-and-modules/) for the exact search order).

## Verifying the install

```bash
marrow --version
# Marrow version: v0.1.0
```

If `marrow --version` works, move on to [Hello World](/getting-started/hello-world/).