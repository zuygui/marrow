---
title: Lints
description: "Style and correctness warnings, and how to turn each one into an error or silence it."
sidebar:
  order: 4
---

A **lint** is a check that the compiler runs on your code, besides the type checker. Each lint has a name and a **level**:

| Level | Effect |
|---|---|
| `allow` | The lint is not reported. |
| `warn` | A warning is printed; the build goes on. |
| `deny` | The lint is an error: it is printed as one and the build stops. |

## The lints

| Name | Code | Default | Reports |
|---|---|---|---|
| `unknown_decorator` | `W0001` | `warn` | A decorator that does not exist. |
| `unreachable_code` | `W0002` | `warn` | Code that can never run. |
| `assign_in_condition` | `W0003` | `warn` | `=` where `==` was probably meant. |
| `shadowing` | `W0004` | `warn` | A variable declared twice in one scope. |
| `unused_result` | `W0005` | `warn` | A result that must be used and is not. |
| `unreachable_arm` | `W0006` | `warn` | A `match` arm that is never selected. |
| `unused` | `W0007` | `warn` | A local variable that is never used. |
| `unused_imports` | `W0008` | `warn` | An `@import` with an alias that is never used. |
| `unused_functions` | `W0009` | `warn` | A `@private` function that is never used. |
| `naming` | `W0010` | `warn` | A name that does not follow the conventions below. |
| `bool_compare` | `W0011` | `warn` | `x == true`, `x != false`, or an `if` that only produces `true` and `false`. |
| `long_function` | `W0012` | `allow` | A function of more than 100 lines. |
| `todo_comment` | `W0013` | `allow` | A `TODO` or `FIXME` in a comment. |
| `unused_parameters` | `W0014` | `allow` | A function parameter that is never used. Prefix the name with `_` to keep it. |

The lints that come with the style rules (`W0007` and later) are only reported for **your own code**: the standard library and your dependencies are never reported, and a `deny` never fails a build because of code that is not yours.

## Changing a level

From the weakest source to the strongest:

1. **The default** from the table above.
2. **`marrow.toml`**, section `[lints]`:

   ```toml
   [lints]
   naming = "deny"
   unused = "warn"
   todo_comment = "warn"
   ```

3. **The command line**, which wins over the file. `-A` is allow, `-W` is warn and `-D` is deny; `all` stands for every lint.

   ```bash
   marrow build -D naming
   marrow check -A unused -W todo_comment
   marrow check -D all -A long_function
   ```

A name that is not a lint is an error, with a suggestion when it is close to a real one.

## Silencing a lint in the code

`@allow(name, ...)` above a function silences the lints in that function, and in front of a local variable it silences them for that variable:

```marrow
@allow(unused)
fn scratch() {
    var a: i64 = 1;
}

fn other() {
    @allow(unused) var b: i64 = 2;
    @allow(naming) var LegacyName: i64 = 3;
    var _c: i64 = 4;        // a leading underscore also means "unused on purpose"
}
```

For a variable, `unused`, `naming` and `shadowing` are the lints that apply. A lint that does not exist in `@allow` is reported as warning `W0001`.

## Naming conventions

| What | Style | Example |
|---|---|---|
| Functions, parameters, local variables, struct fields | `snake_case` | `read_file` |
| Structs, enums and enum variants | `PascalCase` | `HashMap`, `Circle` |
| Global variables | `SCREAMING_SNAKE_CASE` | `MAX_SIZE` |

`@extern` functions keep the name of the symbol they bind to, and names inside a module are judged without their `alias__` prefix. The warning shows the name it would like:

```
warning[W0010]: function `readFile` should be written in snake_case
```

Since the compiler can only tell you the new name, renaming is up to you.

## Other lints

- **`unused`**: assigning to a variable counts as using it. Parameters are not checked.
- **`unused_imports`**: only imports with an alias are checked, because a plain `@import` brings in names that the compiler cannot tell apart.
- **`unused_functions`**: only `@private` functions are checked, and `@export`, `@extern`, `@test` and `main` never are. A function that is only called by itself is not reported.
- **`bool_compare`**: `if x == true` can be written `if x`, and `if c { true } else { false }` is `c`.
- **`long_function`**: counts the lines from `fn` to the closing brace.
