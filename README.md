# Marrow package registry

The index of packages for `marrow add`, `marrow search` and `marrow install`: one file per
package in `packages/`, named after the package (`packages/json.toml`).

```toml
description = "A JSON parser"
repo = "https://github.com/someone/marrow-json"   # any git URL the users can clone

[[version]]
version = "1.0.0"          # must match `package.version` in the repository's marrow.toml
tag = "v1.0.0"             # git tag to fetch (default: "v" + version)

[[version]]
version = "1.1.0"
rev = "0123abc..."         # or pin a commit instead of a tag
# yanked = true            # hide a broken release from new resolutions
```

To publish a package, open a pull request adding (or extending) its file. Package names use
lowercase letters, digits, `-` and `_`. The repository is only cloned when a package is
installed, so the index stays small.
