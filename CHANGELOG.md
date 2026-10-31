# Changelog

## v0.4.0 (unreleased)

### Language: a checked memory model
- Pointer kinds are distinct types: `*T` raw, `^T` owning, `&T` shared borrow, `&mut T` mutable borrow. `&x` borrows, `raw x` takes the raw address.
- Owners are moved (use after move, moves in loops and branches are errors) and freed automatically at the end of their scope. `@drop(fn)` runs a hook when a struct is freed.
- Structs holding a `^T` are owning values: moved, emptied after a move, dropped with their scope.
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

### Standard library
- Vec, String, HashMap, Dir, Fs, inflate/tar buffers, net, TLS chains, Arena, Chan, Cert, Url and HttpMessage are owned (`^T`) and free themselves.
- Dangerous primitives (`realloc`, `dealloc`, `mem_copy`, `*_destroy`, `sys_*`…) are `@unsafe`.

### Known limitations
- A struct value that only has an `@drop` hook (no `^T` field) is not tracked by value.
- A borrow held by a struct built in several steps is not followed.
- Json, Big, Curve/JP and Tls internals still use raw pointers; the compiler's AST stays arena-style.
- `@trusted` is used widely in the standard library and has not been audited.
