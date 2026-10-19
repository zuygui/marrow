#!/bin/sh
# Package manager, offline: the registry and every package are local git repositories.
set -u
root=$(cd "$(dirname "$0")/.." && pwd)
marrow=${MARROW:-$root/build/marrow}
command -v git >/dev/null 2>&1 || { echo "pkg: git not found, skipped"; exit 0; }
tmp=$(cd "$(mktemp -d)" && pwd -P)
trap 'rm -rf "$tmp"' EXIT
fail=0
t() { echo "FAIL $1"; fail=1; }
g() { git -c user.name=t -c user.email=t@t -c init.defaultBranch=main -c commit.gpgsign=false "$@"; }

export MARROW_HOME="$tmp/home"
export MARROW_REGISTRY="$tmp/registry"
export HOME="$tmp/userhome"; mkdir -p "$HOME"
mkdir -p "$tmp/src" "$tmp/bin"
cp -r "$root/std" "$tmp/bin/std"; cp "$marrow" "$tmp/bin/marrow"
marrow="$tmp/bin/marrow"
export PATH="$tmp/bin:$PATH"

# mkpkg NAME VERSION VALUE [dependencies-lines]: a git repository tagged vVERSION
mkpkg() {
    d="$tmp/src/$1"; mkdir -p "$d/src"; cd "$d"
    [ -d .git ] || g init -q
    printf '[package]\nname = "%s"\nversion = "%s"\n\n[dependencies]\n%s\n' "$1" "$2" "${4:-}" > marrow.toml
    cat > src/lib.mw <<MW
@import("helper.mw")
fn value() -> i64 { ret $3 + parse(0); }
fn version_major() -> i64 { ret $(echo "$2" | cut -d. -f1); }
MW
    # the helper's name also exists in the program that uses the package
    printf 'fn parse(x: i64) -> i64 { ret x; }\n' > src/helper.mw
    g add -A; g commit -q -m "release $2"; g tag "v$2"
    cd "$tmp"
}
mkpkg greet 1.0.0 100
mkpkg greet 1.2.0 120
mkpkg greet 2.0.0 200
mkpkg other 0.1.0 7 'greet = "^2"'
mkpkg gitlib 0.1.0 5

# the registry: a repository whose `registry` branch holds packages/<name>.toml
mkdir -p "$tmp/registry/packages/text" "$tmp/registry/packages/misc/tools"; cd "$tmp/registry"
g init -q -b registry
printf 'description = "says hello"\nkeywords = ["greeting", "discord"]\nrepo = "%s"\n\n[[version]]\nversion = "1.0.0"\n[[version]]\nversion = "1.2.0"\n[[version]]\nversion = "2.0.0"\ntag = "v2.0.0"\n' "$tmp/src/greet" > packages/text/greet.toml
printf 'description = "depends on greet 2"\nrepo = "%s"\n\n[[version]]\nversion = "0.1.0"\n' "$tmp/src/other" > packages/misc/tools/other.toml
g add -A; g commit -q -m index
cd "$tmp"

"$marrow" new app >/dev/null 2>&1 || t "new"
[ -f app/marrow.toml ] && [ -f app/src/main.mw ] || t "new files"
cd app

"$marrow" add greet@^1 >/dev/null 2>"$tmp/err" || { t "add greet"; cat "$tmp/err"; }
grep -q 'greet = "\^1"' marrow.toml || t "add writes the requirement"
grep -q 'version = "1.2.0"' marrow.lock || t "highest matching version is locked"
[ -f "$MARROW_HOME/pkg/greet/1.2.0/marrow.toml" ] || t "package cached once per machine"

cat > src/main.mw <<'MW'
@import("std/std.mw")
@import("greet", greet)
@import("other", other)

// both the program and the package have a `parse`: no clash
fn parse(x: i64) -> i64 { ret x + 1; }

@export fn main(argc: i32, argv: rawptr) -> i32 {
    print_i64(greet::value()); print(" "); print_i64(greet::version_major()); print(" ");
    print_i64(other::value()); print(" "); print_i64(parse(1)); println("");
    ret 0;
}
MW
"$marrow" add other >/dev/null 2>"$tmp/err" || { t "add other"; cat "$tmp/err"; }
out=$("$marrow" run 2>"$tmp/err") || { t "run"; cat "$tmp/err"; }
[ "$out" = "120 1 7 2" ] || t "program output: '$out'"
# `other` needs greet 2, the program wants greet 1: both versions are installed, side by side
[ -d "$MARROW_HOME/pkg/greet/1.2.0" ] && [ -d "$MARROW_HOME/pkg/greet/2.0.0" ] || t "two versions installed"
"$marrow" tree 2>/dev/null | grep -q 'greet v2.0.0' || t "tree shows the nested version"

# git and path dependencies
"$marrow" add gitlib --git "$tmp/src/gitlib" --tag v0.1.0 >/dev/null 2>"$tmp/err" || { t "add git"; cat "$tmp/err"; }
grep -q 'gitlib = { git = ' marrow.toml || t "git dependency written"
grep -q 'source = "git+' marrow.lock || t "git dependency locked"
mkdir -p "$tmp/local/src"; printf '[package]\nname = "local"\nversion = "0.0.1"\n' > "$tmp/local/marrow.toml"; echo 'fn five() -> i64 { ret 5; }' > "$tmp/local/src/lib.mw"
"$marrow" add local --path "$tmp/local" >/dev/null 2>&1 || t "add path"
cat > src/main.mw <<'MW'
@import("std/std.mw")
@import("greet", greet)
@import("other", other)
@import("gitlib", gitlib)
@import("local", local)

fn parse(x: i64) -> i64 { ret x + 1; }

@export fn main(argc: i32, argv: rawptr) -> i32 {
    if gitlib::value() + local::five() != 10 { ret 3; }
    if greet::value() + other::value() + parse(0) < 0 { ret 4; }
    ret 0;
}
MW
"$marrow" run >/dev/null 2>"$tmp/err" || { t "git and path dependencies build"; cat "$tmp/err"; }

# the lock file makes builds reproducible and offline
cp marrow.lock "$tmp/lock.before"
mv "$tmp/registry" "$tmp/registry.away"; mv "$MARROW_HOME/registry" "$MARROW_HOME/registry.away" 2>/dev/null
"$marrow" build >/dev/null 2>"$tmp/err" || { t "offline build from lock"; cat "$tmp/err"; }
cmp -s marrow.lock "$tmp/lock.before" || t "lock unchanged by a build"
mv "$tmp/registry.away" "$tmp/registry"; mv "$MARROW_HOME/registry.away" "$MARROW_HOME/registry" 2>/dev/null

# a newer release is only picked up by `update`
mkpkg greet 1.3.0 130
cd "$tmp/registry"; printf '\n[[version]]\nversion = "1.3.0"\n' >> packages/text/greet.toml; g add -A; g commit -q -m 1.3.0
cd "$tmp/app"
"$marrow" install >/dev/null 2>&1; grep -q 'version = "1.3.0"' marrow.lock && t "install keeps the locked version"
"$marrow" update >/dev/null 2>"$tmp/err" || { t "update"; cat "$tmp/err"; }
grep -q 'version = "1.3.0"' marrow.lock || t "update picks the newest compatible version"
"$marrow" search greet 2>/dev/null | grep -q 'greet v2.0.0' || t "search by name"
"$marrow" search discord 2>/dev/null | grep -q 'greet v2.0.0  \[text\]' || t "search by keyword shows the category"
"$marrow" search tools 2>/dev/null | grep -q '^other ' || t "search by folder"
"$marrow" search nomatchatall 2>&1 | grep -q 'no package matches' || t "search without result"

# errors
"$marrow" add nosuchpkg >/dev/null 2>"$tmp/err"; grep -q 'was not found in the registry' "$tmp/err" || t "unknown package message"
"$marrow" add greet@^9 >/dev/null 2>"$tmp/err"; grep -q 'no version of `greet` satisfies' "$tmp/err" || t "unsatisfiable requirement message"
"$marrow" remove gitlib >/dev/null 2>"$tmp/err" || { t "remove"; cat "$tmp/err"; }
grep -q gitlib marrow.toml && t "remove edits the manifest"

cd "$tmp"; mkdir empty; cd empty
"$marrow" build >/dev/null 2>"$tmp/err"; grep -q 'no `marrow.toml` found' "$tmp/err" || t "outside a project"
[ $fail = 0 ] && echo "pkg ok"
exit $fail
