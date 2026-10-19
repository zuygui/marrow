# Marrow package registry

The index behind `marrow add`, `marrow search` and `marrow install`. Packages live in the
`packages/` folder, in sub-folders that act as categories; the file name is the package name:

```
packages/
  chat/
    discord.toml
    irc.toml
  web/
    http-router.toml
  data/
    json/
      json.toml
```

Folders can be nested freely and are only there to organise the index (`marrow search chat`
lists everything under `chat/`). A package name must be unique across the whole tree.

## A package file

```toml
description = "A client for the Discord API"
keywords = ["discord", "bot", "chat"]                # searched by `marrow search`
repo = "https://github.com/someone/marrow-discord"   # any git URL users can clone

[[version]]
version = "1.0.0"          # must match `package.version` in the repository's marrow.toml
tag = "v1.0.0"             # git tag to fetch (default: "v" + version)

[[version]]
version = "1.1.0"
rev = "0123abc..."         # or pin a commit instead of a tag
# yanked = true            # hide a broken release from new resolutions
```

To publish a package, open a pull request adding (or extending) its file. Names use lowercase
letters, digits, `-` and `_`. The repositories are only cloned when a package is installed.
