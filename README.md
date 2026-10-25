# Marrow package registry

The index behind `marrow add`, `marrow search` and `marrow install`.

## Layout

One file per package, in a folder named after the **first two letters** of the package name:

```
packages/
  di/discord.toml
  di/disk-usage.toml
  js/json.toml
  ur/url.toml
```

The path follows directly from the name (`packages/<first two>/<name>.toml`): installing a
package reads one file, and no folder grows large. `marrow search` lists the folders and
matches names without opening the files of the packages that do not match.

## A package file

```toml
description = "A client for the Discord API"
keywords = ["discord", "bot", "chat"]                # searched by `marrow search --deep`
repo = "https://github.com/someone/marrow-discord"   # any git URL users can clone

[[version]]
version = "1.0.0"          # must match `package.version` in the repository's marrow.toml
tag = "v1.0.0"             # git tag to fetch (default: "v" + version)

[[version]]
version = "1.1.0"
rev = "0123abc..."         # or pin a commit instead of a tag
# yanked = true            # hide a broken release from new resolutions
```

To publish a package, open a pull request adding (or extending) its file. Names have at least
two characters: lowercase letters, digits, `-` and `_`. Repositories are only cloned when a
package is installed.

## The index

`index.json` is built from the package files by `scripts/build_index.py` (a GitHub Action runs it
whenever `packages/` changes). It lists every package with its description, keywords, repository
and versions, and is what the website at the documentation's "Registry" page reads in the
browser, so a new package shows up there without republishing anything. Do not edit it by hand.
