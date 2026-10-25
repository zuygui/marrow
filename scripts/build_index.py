#!/usr/bin/env python3
"""Builds index.json from packages/<xx>/<name>.toml.

The index is what the website and `marrow search` read: one small file with everything needed
to list and search the packages. It is rebuilt on every push (see .github/workflows/index.yaml).
"""
import json
import re
import sys
import tomllib
from datetime import datetime, timezone
from pathlib import Path

root = Path(__file__).resolve().parent.parent
errors = []


def version_key(v):
    m = re.match(r"^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?", v)
    if not m:
        return (0, 0, 0, 0, "")
    major, minor, patch, pre = int(m[1]), int(m[2]), int(m[3]), m[4]
    # a release comes after its pre-releases
    return (major, minor, patch, 0 if pre else 1, pre or "")


packages = []
for path in sorted(root.glob("packages/*/*.toml")):
    name = path.stem
    shard = path.parent.name
    if shard != name[:2]:
        errors.append(f"{path.relative_to(root)}: the folder must be `{name[:2]}`")
        continue
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
    except tomllib.TOMLDecodeError as e:
        errors.append(f"{path.relative_to(root)}: {e}")
        continue
    versions = []
    for v in data.get("version", []):
        if "version" not in v or "rev" not in v:
            errors.append(f"{path.relative_to(root)}: a [[version]] needs `version` and `rev`")
            continue
        entry = {"version": str(v["version"]), "rev": str(v["rev"])}
        if "tag" in v:
            entry["tag"] = str(v["tag"])
        versions.append(entry)
    if not versions:
        errors.append(f"{path.relative_to(root)}: no [[version]]")
        continue
    versions.sort(key=lambda v: version_key(v["version"]), reverse=True)
    packages.append(
        {
            "name": name,
            "description": str(data.get("description", "")),
            "keywords": [str(k) for k in data.get("keywords", [])],
            "repo": str(data.get("repo", "")),
            "latest": versions[0]["version"],
            "versions": versions,
        }
    )

if errors:
    print("\n".join(errors), file=sys.stderr)
    sys.exit(1)

index = {
    "format": 1,
    "generated": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "count": len(packages),
    "packages": packages,
}
(root / "index.json").write_text(json.dumps(index, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
print(f"index.json: {len(packages)} package(s)")
