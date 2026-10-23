#!/bin/sh
# Downloading packages without `git`: the sources and the registry come as .tar.gz archives over
# HTTP from a local server (tests/archive/fetchd.mw). `git` is only used here, to build the
# archives; a stand-in `git` in PATH fails the test if marrow ever runs it.
set -u
root=$(cd "$(dirname "$0")/.." && pwd)
marrow=${MARROW:-$root/build/marrow}
GIT=$(command -v git) || { echo "fetch: git not found, skipped"; exit 0; }
tmp=$(cd "$(mktemp -d)" && pwd -P)
srv_pid=""
trap '[ -n "$srv_pid" ] && kill $srv_pid 2>/dev/null; rm -rf "$tmp"' EXIT
fail=0
t() { echo "FAIL $1"; fail=1; }
g() { "$GIT" -c user.name=t -c user.email=t@t -c init.defaultBranch=main -c commit.gpgsign=false "$@"; }
exe=""; case "$(uname -s)" in MINGW*|MSYS*) exe=.exe ;; esac

mkdir -p "$tmp/src" "$tmp/bin" "$tmp/www"
g -C "$tmp" init -q -b probe >/dev/null 2>&1
echo x > "$tmp/probe.txt"; g -C "$tmp" add probe.txt; g -C "$tmp" commit -q -m p
g -C "$tmp" archive --format=tar.gz -o "$tmp/probe.tgz" HEAD 2>/dev/null || { echo "fetch: git archive unavailable, skipped"; exit 0; }

unset HTTP_PROXY http_proxy HTTPS_PROXY https_proxy
export MARROW_HOME="$tmp/home"
export HOME="$tmp/userhome"; mkdir -p "$HOME"
cp -r "$root/std" "$tmp/bin/std"; cp "$marrow" "$tmp/bin/marrow"; marrow="$tmp/bin/marrow"
cp -r "$root/std" "$tmp/std"; cp "$root/tests/archive/fetchd.mw" "$tmp/fetchd.mw"
(cd "$tmp" && "$marrow" fetchd.mw >/dev/null 2>"$tmp/err") || { t "build the test server"; cat "$tmp/err"; exit 1; }

# `git` must never be started: this one only leaves a mark
printf '#!/bin/sh\ntouch "%s/git-was-run"\nexit 99\n' "$tmp" > "$tmp/bin/git"; chmod +x "$tmp/bin/git"
export PATH="$tmp/bin:$PATH"

port=$((20000 + $$ % 20000))
"$tmp/fetchd$exe" $port "$tmp/www" "$tmp/req.log" >/dev/null 2>&1 &
srv_pid=$!
i=0; while [ ! -f "$tmp/req.log.ready" ] && [ $i -lt 100 ]; do sleep 0.1; i=$((i+1)); done
[ -f "$tmp/req.log.ready" ] || { t "test server did not start"; exit 1; }
base="http://127.0.0.1:$port"

mkpkg() {
    d="$tmp/src/$1"; mkdir -p "$d/src"; cd "$d"
    [ -d .git ] || g init -q
    printf '[package]\nname = "%s"\nversion = "%s"\n\n[dependencies]\n' "$1" "$2" > marrow.toml
    printf 'fn value() -> i64 { ret %s; }\n' "$3" > src/lib.mw
    g add -A; g commit -q -m "release $2"; g tag "v$2"
    mkdir -p "$tmp/www/o/$1/archive"
    g archive --format=tar.gz --prefix="$1-v$2/" -o "$tmp/www/o/$1/archive/v$2.tar.gz" "v$2"
    cd "$tmp"
}
mkregistry() {
    rm -rf "$tmp/registry"; mkdir -p "$tmp/registry/packages/gr" "$tmp/registry/packages/gi"; cd "$tmp/registry"
    g init -q -b registry
    { printf 'description = "says hello"\nrepo = "%s/o/greet"\n' "$base"; for v in "$@"; do printf '\n[[version]]\nversion = "%s"\n' "$v"; done; } > packages/gr/greet.toml
    g add -A; g commit -q -m index
    mkdir -p "$tmp/www/reg/archive"
    g archive --format=tar.gz --prefix=reg-registry/ -o "$tmp/www/reg/archive/registry.tar.gz" HEAD
    cd "$tmp"
}
mkpkg greet 1.0.0 100
mkpkg greet 1.2.0 120
mkpkg gitlib 0.1.0 5
mkregistry 1.0.0 1.2.0
export MARROW_REGISTRY="$base/reg"
export MARROW_FETCH=http

"$marrow" new app >/dev/null 2>&1; cd app
"$marrow" add greet@^1 >/dev/null 2>"$tmp/err" || { t "add from the registry"; cat "$tmp/err"; }
grep -q 'version = "1.2.0"' marrow.lock || t "highest version locked"
want=$(g -C "$tmp/src/greet" rev-parse v1.2.0)
grep -q "rev = \"$want\"" marrow.lock || { t "the commit comes from the archive"; cat marrow.lock; }
[ -f "$MARROW_HOME/pkg/greet/1.2.0/marrow.toml" ] && [ ! -d "$MARROW_HOME/pkg/greet/1.2.0/.git" ] || t "package unpacked, without .git"
cat > src/main.mw <<'MW'
@import("std/std.mw")
@import("greet", greet)
@export fn main(argc: i32, argv: rawptr) -> i32 { println_i64(greet::value()); ret 0; }
MW
out=$("$marrow" run 2>"$tmp/err") || { t "run"; cat "$tmp/err"; }
[ "$out" = "120" ] || t "program output: '$out'"
"$marrow" search greet 2>/dev/null | grep -q '^greet v1.2.0  says hello' || t "search through the downloaded registry"

# a git dependency by URL
"$marrow" add gitlib --git "$base/o/gitlib" --tag v0.1.0 >/dev/null 2>"$tmp/err" || { t "git dependency by url"; cat "$tmp/err"; }
want=$(g -C "$tmp/src/gitlib" rev-parse v0.1.0)
grep -q "rev = \"$want\"" marrow.lock || t "git dependency commit"

# the index is only downloaded again when it changed
"$marrow" update >/dev/null 2>&1
grep -q '^304 /reg/archive/registry.tar.gz' "$tmp/req.log" || { t "unchanged registry answers 304"; cat "$tmp/req.log"; }
mkpkg greet 1.3.0 130
mkregistry 1.0.0 1.2.0 1.3.0
cd "$tmp/app"
"$marrow" update >/dev/null 2>"$tmp/err" || { t "update"; cat "$tmp/err"; }
grep -q 'version = "1.3.0"' marrow.lock || t "a changed registry is downloaded again"

# redirects are followed
rm -rf "$MARROW_HOME"
MARROW_REGISTRY="$base/redirect/reg" "$marrow" search greet 2>/dev/null | grep -q '^greet v1.3.0' || t "redirect to the archive"
grep -q '^302 /redirect/reg/archive' "$tmp/req.log" || t "redirect was taken"

# errors are explained
rm -f "$tmp/www/o/greet/archive/v1.0.0.tar.gz"
cd "$tmp"; "$marrow" new bad >/dev/null 2>&1; cd bad
"$marrow" add greet@=1.0.0 >/dev/null 2>"$tmp/err"; grep -q 'HTTP 404' "$tmp/err" || { t "404 message"; cat "$tmp/err"; }
MARROW_FETCH=http "$marrow" add localpkg --git "/tmp/some/local/repo" >/dev/null 2>"$tmp/err"
grep -q 'cannot be downloaded as an archive' "$tmp/err" || t "local paths are refused in http mode"

# automatic mode also prefers archives
unset MARROW_FETCH
cd "$tmp"; rm -rf "$MARROW_HOME"; "$marrow" new auto >/dev/null 2>&1; cd auto
"$marrow" add greet@^1 >/dev/null 2>"$tmp/err" || { t "auto mode"; cat "$tmp/err"; }
[ ! -f "$tmp/git-was-run" ] || t "git was started"

# the real thing, over the internet (CI only): the public registry, and a repository on GitHub
if [ "${MARROW_NET_TESTS:-}" = 1 ]; then
    unset MARROW_REGISTRY MARROW_FETCH
    export MARROW_HOME="$tmp/nethome"
    "$marrow" search >/dev/null 2>"$tmp/err" || { t "real registry"; cat "$tmp/err"; }
    [ -d "$MARROW_HOME/registry" ] || t "the real registry was downloaded"
    cd "$tmp"; "$marrow" new net >/dev/null 2>&1; cd net
    printf '[package]\nname = "net"\nversion = "0.1.0"\n\n[dependencies]\nmw = { git = "https://github.com/zuygui/marrow", tag = "v0.2.0" }\n' > marrow.toml
    "$marrow" install >/dev/null 2>"$tmp/err"     # the repository is not a package: the download is what matters
    d=$(ls -d "$MARROW_HOME"/git/*/637daa23497db6e582230feda261f537e623a79e 2>/dev/null | head -1)
    [ -n "$d" ] && [ -f "$d/compiler/marrow.mw" ] || { t "real repository downloaded"; cat "$tmp/err"; }
    [ ! -f "$tmp/git-was-run" ] || t "git was started (network)"
fi
[ $fail = 0 ] && echo "fetch ok"
exit $fail
