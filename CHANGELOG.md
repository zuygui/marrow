# Changelog

## v0.5.0 "Differentiation" (unreleased)

### Language: methods and inheritance
- `impl Type { ... }` blocks declare associated functions. A first parameter `self`, `&self`, `&mut self` or `^self` makes a method: `p.norm()` borrows `p` the way the signature asks. Functions without `self` are called as `Type::new(...)`. Several `impl` blocks per type are allowed.
- Single inheritance by composition: `struct Dog : Animal { ... }` places a field `base: Animal` first. Inherited fields (`dog.age`) and methods (`dog.birthday()`) are reachable on the child, a method of the child hides the parent's one, and `&Dog` converts to `&Animal`. Calls are resolved statically (there is no virtual dispatch).
- Generic structs have methods too: `impl<T> Pair<T> { fn first(&self) -> T { ... } }`. The type parameters of the block are the ones of the struct, in the same order; a method of such a block cannot add its own. Each instance (`Pair<i64>`) gets its methods.
- `impl` and `self` are not reserved words: they only mean something at the places above.

### Language: memory model
- A struct that holds a `@drop` struct by value, at any depth, is an owning value: the hook of the field runs once when the outer value goes out of scope, and not after it was moved.
- A borrow that a callee may store through a `&mut` argument is followed: after `keep(&mut holder, &x)`, `holder` keeps `x` borrowed.

### Performance
- `x / 2^k` and `x % 2^k` with a literal divisor compile to shifts and masks instead of `idiv` (rounding toward zero for signed values). The Collatz benchmark runs about twice as fast.
- Small non-recursive functions with scalar parameters and result (body of at most 40 AST nodes) are inlined at their direct call sites. A loop calling two tiny helpers runs about 2.7x faster; the six benchmark programs contain almost no such helpers, so their times are unchanged.
- Conditions made of `&&`, `||` and `!` (in `if`, `while`, `for`) compile to direct branches instead of materializing a boolean in a stack slot. The generated IL is smaller; run times on the benchmarks are unchanged.
- Functions that are not reachable from an `@export` function (or from a `@drop` hook or a constant) are no longer emitted. The IL of the benchmark programs shrinks by about a third, which brings the compile time back to the level of `gcc -O0`. A library with no `@export` function keeps all of its functions.
- A recursive function of at most 30 AST nodes is inlined into itself two levels deep: `fib(35)` runs in about half the time.
- `benchmarks/run.py --exec` also times the compiled programs.

### Fixes
- The naming lint no longer reports generic instances (`Pair<i64>` was flagged as a struct named `i64`).

### Tooling
- Lint `unneeded_trusted` (W0015, off by default): a `@trusted` function whose body does nothing that needs it. Use it to audit `@trusted`.

### Known limitations
- A `@drop` struct held in an array is still not tracked by value.
- Containers stored in structs or globals (including the compiler's own data) and the AST stay raw pointers; only single-owner locals in the compiler are owned.
- `impl` for enums only gives associated functions (`Color::code(c)`): enum values are plain integers, so `c.code()` is not available. There is no virtual dispatch.
- `@trusted` is used widely in the standard library (370 functions) and has not been fully audited.

## v0.4.0 "Homeostasis" (unreleased)

### Language: a checked memory model
- Pointer kinds are distinct types: `*T` raw, `^T` owning, `&T` shared borrow, `&mut T` mutable borrow. `&x` borrows, `raw x` takes the raw address.
- Owners are moved (use after move, moves in loops and branches are errors) and freed automatically at the end of their scope. `@drop(fn)` runs a hook when a struct is freed.
- Structs holding a `^T` are owning values: moved, emptied after a move, dropped with their scope.
- A struct with a `@drop` hook is an owning value (variables and parameters): moved, hook run once at the end of its scope.
- A borrow stored in a field of a local struct (`h.r = &x`) keeps its target borrowed.
- Borrows are checked (shared xor mutable, lifetimes follow blocks). A borrow that outlives its target (returned, stored in a struct, kept behind a pointer) is rejected.
- `const` bindings and an opt-in `std/option.mw`.
- Giving an allocation up is explicit: `cast(*T) owner` moves the owner and leaves the memory to the program.
- Owners and borrows decay to `*T` when passed as call arguments.

### Language: the unsafe boundary
- `unsafe { }` blocks are required for raw addresses, raw dereferences, indexing and arithmetic on raw pointers, reading `^T`/`&T` fields through a raw struct pointer, and casts to `^T`/`&T`/`&mut T`.
- `@unsafe` functions and `@extern` functions can only be called from an unsafe context.
- `@trusted` functions may use unsafe operations in their body and are safe to call.

### Tooling
- `marrow fmt` (comment-preserving formatter), `marrow doc` (Markdown from `///`), `marrow lsp` (diagnostics and formatting), `--message-format json`.
- Lint `unused_parameters` (W0014), off by default.
- `marrow help` shows a banner with the version (and the release name once there is one); plain ASCII when piped, colored on a terminal, `--color` and `NO_COLOR` respected.

### Standard library
- Vec, String, HashMap, Dir, Fs, inflate/tar buffers, net, TLS chains, Arena, Chan, Cert, Url, HttpMessage, Json, Big, Tls, Curve and JP are owned (`^T`) and free themselves.
- Dangerous primitives (`realloc`, `dealloc`, `mem_copy`, `*_destroy`, `sys_*`…) are `@unsafe`.

### Known limitations
- A hook-only struct nested inside another struct or an array is not tracked by value.
- A borrow that a function stores into a `&mut` parameter is not followed.
- Containers stored in structs or globals (including the compiler's own data) and the AST stay raw pointers; only single-owner locals in the compiler are owned.
- `@trusted` is used widely in the standard library and has not been audited.
