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
mkdir -p "$tmp/registry/packages/gr" "$tmp/registry/packages/ot"; cd "$tmp/registry"
g init -q -b registry
printf 'description = "says hello"\nkeywords = ["greeting", "discord"]\nrepo = "%s"\n\n[[version]]\nversion = "1.0.0"\n[[version]]\nversion = "1.2.0"\n[[version]]\nversion = "2.0.0"\ntag = "v2.0.0"\n' "$tmp/src/greet" > packages/gr/greet.toml
printf 'description = "depends on greet 2"\nrepo = "%s"\n\n[[version]]\nversion = "0.1.0"\n' "$tmp/src/other" > packages/ot/other.toml
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
cd "$tmp/registry"; printf '\n[[version]]\nversion = "1.3.0"\n' >> packages/gr/greet.toml; g add -A; g commit -q -m 1.3.0
cd "$tmp/app"
"$marrow" install >/dev/null 2>&1; grep -q 'version = "1.3.0"' marrow.lock && t "install keeps the locked version"
"$marrow" update >/dev/null 2>"$tmp/err" || { t "update"; cat "$tmp/err"; }
grep -q 'version = "1.3.0"' marrow.lock || t "update picks the newest compatible version"
"$marrow" search greet 2>/dev/null | grep -q 'greet v2.0.0  says hello' || t "search by name"
"$marrow" search REE 2>/dev/null | grep -q '^greet ' || t "search by part of a name, any case"
"$marrow" search 2>/dev/null | grep -c . | grep -q '^2$' || t "search without text lists everything"
"$marrow" search discord 2>&1 | grep -q 'no package matches' || t "keywords are not searched by default"
"$marrow" search --deep discord 2>/dev/null | grep -q '^greet ' || t "deep search finds keywords"
[ -f "$MARROW_HOME/registry"/*/packages/gr/greet.toml ] || t "index path follows from the name"

# errors
"$marrow" add nosuchpkg >/dev/null 2>"$tmp/err"; grep -q 'was not found in the registry' "$tmp/err" || t "unknown package message"
"$marrow" add greet@^9 >/dev/null 2>"$tmp/err"; grep -q 'no version of `greet` satisfies' "$tmp/err" || t "unsatisfiable requirement message"
"$marrow" remove gitlib >/dev/null 2>"$tmp/err" || { t "remove"; cat "$tmp/err"; }
grep -q gitlib marrow.toml && t "remove edits the manifest"

# ---- outdated, cache -------------------------------------------------------------------------
cd "$tmp/app"
sed 's/^greet = .*/greet = "^1"/' marrow.toml > marrow.toml.new && mv marrow.toml.new marrow.toml
"$marrow" outdated >"$tmp/out" 2>"$tmp/err" || { t "outdated"; cat "$tmp/err"; }
grep -q '^greet  *1\.3\.0  *-  *2\.0\.0' "$tmp/out" || { t "outdated shows current, compatible and latest"; cat "$tmp/out"; }
"$marrow" cache path 2>/dev/null | grep -q "$MARROW_HOME" || t "cache path"
"$marrow" cache 2>/dev/null | grep -q 'package versions' || t "cache list"
"$marrow" cache bogus >/dev/null 2>"$tmp/err"; grep -q 'unknown cache command' "$tmp/err" || t "cache unknown command"

# ---- dev-dependencies and `marrow test` --------------------------------------------------------
cd "$tmp"; "$marrow" new tapp >/dev/null 2>&1; cd tapp
printf '\n[dev-dependencies]\ngreet = "^1"\n' >> marrow.toml
mkdir tests
cat > src/lib.mw <<'MW'
fn add(a: i64, b: i64) -> i64 { ret a + b; }
MW
cat > tests/unit.mw <<'MW'
@import("std/std.mw")
@import("std/test.mw")
@import("greet", greet)
@import("../src/lib.mw", lib)

@test fn adds() { check_eq_i64(lib::add(1, 2), 3, "1+2"); }
@test fn uses_dev_dependency() { check(greet::value() > 0, "greet::value"); }
@test fn broken() { check_eq_i64(lib::add(1, 1), 3, "1+1 is not 3"); }
MW
cat > tests/prog.mw <<'MW'
@import("std/std.mw")
@export fn main(argc: i32, argv: rawptr) -> i32 { ret 0; }
MW
"$marrow" test >"$tmp/out" 2>"$tmp/err"; rc=$?
[ $rc = 1 ] || t "marrow test fails when a test fails (rc=$rc)"
grep -q 'test unit::adds ... ok' "$tmp/out" || { t "test passes"; cat "$tmp/out" "$tmp/err"; }
grep -q 'test unit::uses_dev_dependency ... ok' "$tmp/out" || t "dev-dependencies are visible to tests"
grep -q 'test unit::broken ... FAILED' "$tmp/out" || t "test failure reported"
grep -q '1+1 is not 3' "$tmp/out" || t "failure output shown"
grep -q 'test prog ... ok' "$tmp/out" || t "a tests/*.mw program is a test"
grep -q 'test result: FAILED. 3 passed; 1 failed' "$tmp/out" || t "summary"
"$marrow" test adds >"$tmp/out" 2>&1 || t "filtered test run"
grep -q 'test result: ok. 1 passed' "$tmp/out" || t "filter selects tests"
"$marrow" test nomatch >/dev/null 2>"$tmp/err"; grep -q 'no test matches' "$tmp/err" || t "filter without match"
# dev-dependencies are not available to the build
cat > src/main.mw <<'MW'
@import("std/std.mw")
@import("greet", greet)
@export fn main(argc: i32, argv: rawptr) -> i32 { ret 0; }
MW
"$marrow" build >/dev/null 2>"$tmp/err" && t "dev-dependency is not importable by the build"
# and users of a package do not install its dev-dependencies
cd "$tmp"; "$marrow" new user >/dev/null 2>&1; cd user
rm -rf "$MARROW_HOME/pkg/greet"
"$marrow" add x --path "$tmp/tapp" >/dev/null 2>&1
[ -d "$MARROW_HOME/pkg/greet" ] && t "dev-dependencies of a dependency are not installed"

# ---- publish -----------------------------------------------------------------------------
cd "$tmp"; rm -rf pubpkg origin.git; "$marrow" new pubpkg --lib >/dev/null 2>&1; cd pubpkg
printf '[package]\nname = "pubpkg"\nversion = "0.2.0"\ndescription = "a published package"\nkeywords = ["demo"]\n' > marrow.toml
g init -q; g add -A; g commit -q -m init
"$marrow" publish >/dev/null 2>"$tmp/err"; grep -q 'tag `v0.2.0` does not exist' "$tmp/err" || t "publish needs a tag"
g tag v0.2.0
g init -q --bare "$tmp/origin.git"; g remote add origin "$tmp/origin.git"
"$marrow" publish >/dev/null 2>"$tmp/err"; grep -q 'not pushed' "$tmp/err" || t "publish needs a pushed tag"
g push -q origin v0.2.0 2>/dev/null
"$marrow" publish >/dev/null 2>"$tmp/err" || { t "publish"; cat "$tmp/err"; }
e=build/publish/packages/pu/pubpkg.toml
[ -f "$e" ] || t "publish writes the sharded registry entry"
grep -q 'description = "a published package"' "$e" && grep -q "repo = \"$tmp/origin.git\"" "$e" && grep -q 'version = "0.2.0"' "$e" && grep -q '^rev = "' "$e" || { t "entry content"; cat "$e"; }
printf '[package]\nname = "pubpkg"\nversion = "0.2.0"\n[dependencies]\nx = { path = "../x" }\n' > marrow.toml
"$marrow" publish >/dev/null 2>"$tmp/err"; grep -q 'local path' "$tmp/err" || t "publish refuses path dependencies"

# ---- incremental builds -----------------------------------------------------------------------
cd "$tmp"; "$marrow" new inc >/dev/null 2>&1; cd inc
"$marrow" build >/dev/null 2>"$tmp/err" || { t "first build"; cat "$tmp/err"; }
grep -q 'Compilation successful' "$tmp/err" || t "first build compiles"
"$marrow" build >/dev/null 2>"$tmp/err"
grep -q 'Up to date' "$tmp/err" || { t "second build is skipped"; cat "$tmp/err"; }
sed 's/Hello/Bye/' src/main.mw > src/main.new && mv src/main.new src/main.mw
"$marrow" build >/dev/null 2>"$tmp/err"
grep -q 'Compilation successful' "$tmp/err" || t "changed source is rebuilt"
rm -f build/inc build/inc.exe
"$marrow" build >/dev/null 2>"$tmp/err"
grep -q 'Compilation successful' "$tmp/err" || t "missing executable is rebuilt"
[ "$("$marrow" run 2>/dev/null)" = "Bye from inc!" ] || t "rebuilt program runs"

# ---- why, vendor, clean, completions, verbose ------------------------------------------------
cd "$tmp/app"
"$marrow" why greet >"$tmp/out" 2>&1 || t "why"
grep -q 'app -> greet v' "$tmp/out" || { t "why shows a chain"; cat "$tmp/out"; }
"$marrow" why nosuch >/dev/null 2>"$tmp/err"; grep -q 'not in the dependency tree' "$tmp/err" || t "why unknown package"
"$marrow" vendor >/dev/null 2>"$tmp/err" || { t "vendor"; cat "$tmp/err"; }
[ -f vendor/greet/1.3.0/marrow.toml ] && [ ! -d vendor/greet/1.3.0/.git ] || t "vendor copies packages"
# the vendor folder is enough: no cache, no registry
mv "$MARROW_HOME" "$tmp/home.away"; mv "$tmp/registry" "$tmp/registry.away"
"$marrow" tree >/dev/null 2>"$tmp/err" || { t "vendored tree works offline"; cat "$tmp/err"; }
mv "$tmp/home.away" "$MARROW_HOME"; mv "$tmp/registry.away" "$tmp/registry"
"$marrow" clean >/dev/null 2>&1; [ ! -d build ] || t "clean removes build"
"$marrow" completions bash | grep -q 'complete -o default -F _marrow marrow' || t "bash completions"
"$marrow" completions zsh | grep -q compdef || t "zsh completions"
"$marrow" completions fish | grep -q 'complete -c marrow' || t "fish completions"
"$marrow" completions powershell | grep -q Register-ArgumentCompleter || t "powershell completions"
"$marrow" completions tcsh >/dev/null 2>"$tmp/err"; grep -q 'unknown shell' "$tmp/err" || t "unknown shell"
cd "$tmp/inc"; rm -rf build
"$marrow" build -v 2>&1 | grep -q 'Running.*qbe' || t "verbose shows qbe command"

# ---- build progress ------------------------------------------------------------------------
cd "$tmp"; "$marrow" new prog >/dev/null 2>&1; cd prog
"$marrow" add greet@^1 >/dev/null 2>&1
cat > src/main.mw <<'MW'
@import("std/std.mw")
@import("greet", greet)
@export fn main(argc: i32, argv: rawptr) -> i32 { println_i64(greet::value()); ret 0; }
MW
MARROW_PROGRESS=plain "$marrow" build >/dev/null 2>"$tmp/err" || { t "build with plain progress"; cat "$tmp/err"; }
grep -q 'Compiled greet v' "$tmp/err" && grep -q 'Compiled prog v0.1.0' "$tmp/err" && grep -q 'Compiled std' "$tmp/err" || { t "plain progress lists the packages"; cat "$tmp/err"; }
grep -q 'Linking' "$tmp/err" || t "plain progress shows linking"
rm -rf build; MARROW_PROGRESS=never "$marrow" build >/dev/null 2>"$tmp/err"
grep -q 'Compiled' "$tmp/err" && t "progress can be turned off"
rm -rf build; MARROW_PROGRESS=always "$marrow" build >/dev/null 2>"$tmp/err"
grep -q 'Finished prog v0.1.0' "$tmp/err" || { t "live progress ends with a summary"; cat -v "$tmp/err" | tail -3; }
rm -rf build; "$marrow" build >/dev/null 2>"$tmp/err"
grep -q 'Compilation successful' "$tmp/err" || t "no progress display when stderr is not a terminal"

cd "$tmp"; mkdir empty; cd empty
"$marrow" build >/dev/null 2>"$tmp/err"; grep -q 'no `marrow.toml` found' "$tmp/err" || t "outside a project"
[ $fail = 0 ] && echo "pkg ok"
exit $fail
