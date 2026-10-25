---
title: Projects and packages
description: marrow.toml, dependencies, the registry, marrow.lock, vendoring and publishing.
sidebar:
  order: 2
---

Marrow has a package manager built into the `marrow` command. A **project** is a folder with a `marrow.toml`; a **package** is a project that other projects import.

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

## Using packages

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
- **The registry** is the `registry` branch of <https://github.com/zuygui/marrow>: one file `packages/<first two letters>/<name>.toml` per package. `search` and `outdated` read it, and the [registry page](/marrow/registry/) of this site lists and searches the packages from the same data: it reads the registry's `index.json` in your browser, so a new package shows up there without the site being rebuilt. `MARROW_REGISTRY` and `MARROW_REGISTRY_BRANCH` point to another one.

## Publishing

`marrow publish` checks that the project can be published and writes the registry entry to `build/publish/packages/<xx>/<name>.toml`. Add that file to the `registry` branch with a pull request. `--tag` names the git tag of the release and `--repo` the repository URL.

## Understanding the tree

```bash
marrow tree          # the dependency tree
marrow why greet     # every chain from the project to `greet`
marrow outdated      # dependencies with a newer version
```


See the [CLI reference](/marrow/cli/overview/) for every command and option, and [Modules & decorators](/marrow/language/decorators-and-modules/#namespaces) for how `@import("name", alias)` works.
