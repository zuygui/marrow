---
title: CLI reference
description: Everything the marrow command-line tool does: compile programs, manage packages, run tests.
sidebar:
  order: 1
---

The `marrow` binary is the compiler front-end, the driver that invokes `qbe` and your system C toolchain, and the package manager. Run `marrow help` for the short version of this page.

```
marrow <COMMAND> [OPTIONS]
marrow [OPTIONS] <input.mw> [output.ssa]     (same as `marrow build`)
```

Giving a `.mw` file with no command still works exactly as before: it is `marrow build <input.mw>`.

## Commands

### Building

| Command | Description |
|---|---|
| `build [input.mw]` | Compile a program. With no file, the project's `main` is built. |
| `run [input.mw] [-- ARGS...]` | Build, then run the program. What follows `--` is passed to it. |
| `check [input.mw]` | Type-check only. No code is generated, so neither `qbe` nor a C compiler is needed. |

### Projects and packages

| Command | Description |
|---|---|
| `new <name> [--lib]` | Create a project in a new directory. |
| `init [--lib]` | Create a project in the current directory. |
| `add <name>[@version] [--git URL [--tag T \| --rev R \| --branch B]] [--path DIR]` | Add a dependency to `marrow.toml` and install it. |
| `remove <name>` | Remove a dependency. |
| `install` | Fetch the dependencies listed in `marrow.toml`. |
| `update` | Fetch newer versions of the dependencies and rewrite `marrow.lock`. |
| `tree` | Show the dependency tree. |
| `why <name>` | Show every chain of dependencies that leads to a package. |
| `outdated` | Show the dependencies that have a newer version. |
| `search [text]` | Search the package registry. |
| `vendor` | Copy the dependencies into `vendor/`, so that builds work offline. |
| `publish [--tag T] [--repo URL]` | Check the project and write its registry entry. |
| `test [filter]` | Run the tests. |
| `clean` | Remove the `build/` folder. |
| `cache [list\|path\|clean]` | Show or empty the package cache. |

### Other

| Command | Description |
|---|---|
| `doctor` | Check that the tools needed to build programs (`qbe`, a C compiler) are installed. |
| `completions <shell>` | Print a completion script for `bash`, `zsh`, `fish` or `powershell`. |
| `version` | Print the compiler version. |
| `help` | Print the usage text. |

## Options

| Option | Description |
|---|---|
| `--check` | Type-check the program and stop (the same as the `check` command). |
| `--nostdlib` | Link without libc: the program uses Marrow's own `_start` and system-call stub (see [Cross-compilation](/marrow/guides/cross-compilation/)). The whole [standard library](/marrow/standard-library/overview/) works in this mode. |
| `--target <arch>-<os>` | Compile for another target. `arch` is `x86_64`, `aarch64` or `riscv64`; `os` is `linux`, `macos` or `windows`. Defaults to the host. |
| `-v`, `--verbose` | Print the `qbe` and C compiler commands that are run. |
| `--color <when>` | Colorize diagnostics: `auto` (default), `always` or `never`. |
| `-V`, `--version` | Print `Marrow version: v<version>` and exit. |
| `-h`, `--help` | Print the usage text and exit. |

Running `marrow` with no input and no command prints the usage text to stderr and exits with code `2`.

## What a build does, in order

1. **Existence check.** If the input file doesn't exist, prints an error and exits with code `2`.
2. **Parse and resolve imports.** Lexes and parses the entry file, follows every `@import(...)` (expanding `{target}` to the target's platform name, e.g. `linux_x86_64`) and builds the program. See [Modules & decorators](/marrow/language/decorators-and-modules/).
3. **Type checking.** Reports errors with source excerpts, labels, notes and suggested fixes. Any error stops the build with exit code `1`. With `check` the compiler stops successfully here.
4. **Code generation.** Walks the program and writes QBE IL to the `.ssa` file.
5. **QBE.** Runs `qbe -t <target> <output>.ssa -o <output>.s`. `qbe` must be on your `PATH`.
6. **Runtime assembly.** The generated assembly is extended with Marrow's runtime: the `__marrow_syscall` stub, the context-switch routine used by [tasks](/marrow/guides/async/), and, with `--nostdlib` on Linux, the `_start` entry point.
7. **Cross targets stop here.** When `--target` names something other than the host, `marrow` leaves the `.s` file and prints where it is; assembling and linking is up to you (see [Cross-compilation](/marrow/guides/cross-compilation/)).
8. **Assemble and link** with `cc` (`gcc` on Windows):
   - **Library** (`@no_main` somewhere in the program): `cc -c` to a `.o` file, then prints how to link it.
   - **Program:** links an executable (`-nostdlib -static` for freestanding Linux builds, `-lws2_32` on Windows).

Builds are **incremental**: when nothing that the program depends on has changed since the last build, `marrow` says the program is up to date and skips `qbe` and the linker.

### Build progress

In a terminal, a build shows a progress bar and a line for every package that has been compiled:

```
    Compiled greet v1.2.0  (14 functions)
    Compiled std  (1203 functions)
    Linking
    Finished app v0.1.0 in 1.4 s  build/app  (1 warning)
```

The bar covers type checking, code generation, assembly and linking. When the output is not a terminal (a pipe, a CI log), the display is off and the usual `Compilation successful!` line is printed. `MARROW_PROGRESS` changes that:

| Value | Effect |
|---|---|
| `auto` | The bar and colors in a terminal, plain output otherwise. This is the default. |
| `plain` | The per-package lines and stages, with no bar and no colors. |
| `always` | The bar even when the output is not a terminal. |
| `never` | No progress display at all. |

Set `MARROW_ASCII` to draw the bar with `#` and `-` instead of block characters.

## Projects

`marrow new app` creates this:

```
app/
├── marrow.toml
├── .gitignore
└── src/main.mw
```

`marrow new lib --lib` creates a library whose entry file is `src/lib.mw`. A project is any folder with a `marrow.toml`; the commands that need one look for it in the current directory and its parents.

```toml
[package]
name = "app"
version = "0.1.0"
# main = "src/main.mw"     # program entry (default)
# lib  = "src/lib.mw"      # what other packages import (default)

[dependencies]
json = "^1.2"                                          # from the registry
util = { git = "https://host/user/util", tag = "v1" }  # any git repository
local = { path = "../local" }                          # a directory

[dev-dependencies]
helper = "^0.3"     # only the project's own tests see these
```

Package names have at least 2 characters. A git dependency takes one of `tag`, `rev` or `branch`.

## Packages

A package is a project that other projects import. Its sources are used from a program with a namespace:

```marrow
@import("std/std.mw")
@import("json", json)

@export fn main(argc: i32, argv: rawptr) -> i32 {
    var v: json::Value = json::parse("[1, 2]");
    ...
}
```

Version requirements follow semantic versioning: `"^1.2"` (compatible), `"~1.2"`, `">=1.0, <2"`, an exact `"1.2.3"` or `"*"`. Resolution picks the newest version that satisfies every requirement in the dependency tree. Several versions of one package can coexist when their requirements cannot be met by a single one.

- **`marrow.lock`** pins the resolved versions and commits. Commit it for programs. `install` and `build` follow it, and only `update` moves it.
- **The cache** is `~/.marrow` (`pkg/`, `git/`, `registry/`, `tmp/`), shared by every project on the machine. `MARROW_HOME` moves it. `marrow cache list` shows what is in it, `cache path` prints where it is and `cache clean` empties it.
- **`vendor/`**: after `marrow vendor`, every registry and git dependency is copied to `vendor/<name>/<version or commit>`. A package that is there is used instead of the cache and the network, so the build works offline.
- **Downloads** need no `git`. Packages and the registry come as archives over HTTPS (GitHub, GitLab, Codeberg and any host that serves `<repo>/archive/<ref>.tar.gz`), and the archive is unpacked by Marrow itself. `git` is only used as a fallback for local paths and SSH URLs, and by `publish`. A private GitHub repository needs a token in `MARROW_TOKEN`. The proxy variables `HTTPS_PROXY` and `NO_PROXY` are honored, and `SSL_CERT_FILE` names extra trusted certificates.
- **The registry** is the `registry` branch of <https://github.com/zuygui/marrow>: one file `packages/<first two letters>/<name>.toml` per package. `search` and `outdated` read it. `MARROW_REGISTRY` and `MARROW_REGISTRY_BRANCH` point to another one.

### Publishing

`marrow publish` checks that the project can be published and writes the registry entry to `build/publish/packages/<xx>/<name>.toml`. Add that file to the `registry` branch with a pull request. `--tag` names the git tag of the release and `--repo` the repository URL.

### Understanding the tree

```bash
marrow tree          # the dependency tree
marrow why greet     # every chain from the project to `greet`
marrow outdated      # dependencies with a newer version
```

## `marrow test`

`marrow test [filter]` runs:

- every `@test fn name() { ... }` found in `src/` and `tests/`: each one runs in its own process, and a failed check ends it with status `1`;
- every program with a `main` in `tests/`: it passes when it exits with status `0`.

`[dev-dependencies]` are available while testing. The filter keeps the tests whose name contains it. The output ends with `test result: ok. N passed; M failed`, and the exit code is `1` when something failed. See [`@test`](/marrow/language/decorators-and-modules/#test) and `std/test.mw`.

```marrow
@import("std/std.mw")
@import("std/test.mw")

@test fn adds() {
    check_eq_i64(1 + 1, 2, "one plus one");
}
```

## Shell completions

```bash
marrow completions bash > ~/.local/share/bash-completion/completions/marrow
marrow completions zsh  > ~/.zfunc/_marrow
marrow completions fish > ~/.config/fish/completions/marrow.fish
marrow completions powershell | Out-String | Invoke-Expression
```

## Environment

| Variable | Effect |
|---|---|
| `MARROW_QBE` | Path of the `qbe` binary (default: next to `marrow`, then `PATH`). |
| `MARROW_CC` | C compiler used to assemble and link (default: `cc`, `gcc` or `clang`). |
| `MARROW_HOME` | Where packages are cached (default: `~/.marrow`). |
| `MARROW_PROGRESS` | Build progress display: `auto`, `plain`, `always` or `never`. |
| `MARROW_ASCII` | Draw the progress bar with ASCII characters. |
| `MARROW_REGISTRY` | URL of the package registry repository. |
| `MARROW_REGISTRY_BRANCH` | Branch of the registry (default: `registry`). |
| `MARROW_FETCH` | How sources are downloaded: `auto` (archives over HTTPS, then `git`), `http` or `git`. |
| `MARROW_TOKEN` | GitHub token used to download private repositories. |
| `HTTPS_PROXY`, `NO_PROXY` | Proxy used for downloads. `SSL_CERT_FILE` names extra trusted certificates. |

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success (or `version` / `help`). |
| `1` | A compile-time error (parse, import or type error, code generation), a `qbe` failure, a linker failure or a failed test. |
| `2` | A usage error: missing argument, unknown command, option or target, input file not found, or failure to write the output. |

## Requirements

- `check` needs nothing but `marrow` itself.
- A full build needs the `qbe` binary and a C compiler (`cc` on Linux/macOS, `gcc` on Windows) on `PATH` for the final assemble + link step. `marrow doctor` tells you what is missing. Without a supported compiler you still get the `.ssa` and `.s` files.
- Cross builds need `qbe` plus whatever assembler and linker targets that platform (for example `clang` and `ld.lld`).

## Examples

```bash
# Check a program without building anything
marrow check app.mw

# A program with no libc at all (Linux)
marrow --nostdlib app.mw

# Assembly for 64-bit ARM Linux
marrow --nostdlib --target aarch64-linux app.mw
clang --target=aarch64-linux-gnu -nostdlib -static -fuse-ld=lld app.s -o app

# A library: std/std.mw is decorated @no_main, so this produces an object file
marrow std/std.mw
# -> std/std.o

# A project with a dependency
marrow new app && cd app
marrow add json
marrow run
```

This is how the project's CI works: every program in `tests/run/` is built with `--nostdlib`, run, and compared with its expected output, on Linux, macOS and Windows runners and (for aarch64 and riscv64) under qemu-user.
