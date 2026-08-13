---
title: Overview & lexical structure
description: Comments, literals, identifiers, operators, and the overall shape of a Marrow program.
sidebar:
  order: 1
---

This page covers the low-level building blocks of Marrow source code: how it's tokenized, and how a program is structured at the top level. It's the ground truth from the compiler's lexer (`lexer.rs`) and parser (`parser.rs`).

## File extension

Marrow source files use the `.mw` extension (the compiler's import resolver also recognizes the older `.mrw` extension as a fallback, but everything in the standard library and tooling uses `.mw`).

## Program structure

A Marrow file is a flat sequence of **global items**. Each global item is:

```
(@decorator)* (fn | struct | var) ...
```

i.e. zero or more [decorators](/language/decorators-and-modules/) followed by exactly one of:

- a **function** declaration (`fn`)
- a **struct** declaration (`struct`)
- a **global variable/constant** declaration (`var`)

There is no other kind of top-level statement — no bare expressions, no top-level `if`, nothing like that.

## Comments

```marrow
// a line comment, runs to the end of the line

/* a block comment,
   can span multiple lines */
```

Block comments do **not** nest — the first `*/` closes the comment. An unterminated `/* ...` is a compile error.

## Identifiers

`[A-Za-z_][A-Za-z0-9_]*` — letters, digits and underscores, not starting with a digit. Identifiers are case-sensitive.

## Literals

| Kind | Examples | Notes |
|---|---|---|
| Integer | `0`, `42`, `1000000` | Parsed as a 128-bit signed integer internally, then narrowed/coerced to the target type. |
| Hexadecimal integer | `0x1A`, `0xFF` | `0x`/`0X` prefix, hex digits only. |
| Float | `3.14`, `0.5` | Must have digits on both sides of the `.` (a bare `3.` is not a valid float literal). |
| String | `"hello\n"` | Double-quoted. Supported escapes: `\n \t \r \0 \\ \" \'`. Strings are compiled as `*u8` pointers to null-terminated, interned static data. |
| Char | `'a'`, `'\n'` | Single-quoted, same escapes as strings. A char literal has type `u8`. |
| Bool | `true`, `false` | Keywords, not a distinct token kind — recognized by the parser. |
| Null | `null` | The null pointer constant, of type `rawptr` (coerces to any pointer type). |

## Operators & punctuation

```
::  :  ;  ,  .  ..  ...
(  )  {  }  [  ]
->  =>  @
=  ==  !=  !
<  <=  >  >=
+  +=  -  -=  *  *=  /  /=  %
&&  ||  &
```

Notable points:

- `&` is the address-of / bitwise-and-ish operator (address-of when unary, logical operand construction is done via `&&` for boolean "and" — there is **no** bitwise-only `|` operator; only `||` exists as a two-character token, a lone `|` is a lexer error).
- `*` is overloaded between "multiply" (binary), "pointer type" (in type position, prefix) and "dereference" (unary, in expression position) — see [Pointers, arrays & slices](/language/pointers-arrays-slices/).
- `::` is tokenized but not currently used by any grammar rule in the parser — reserved for future use (e.g. namespacing).

## Keywords

The lexer does not have a separate keyword table — keywords are plain identifiers that the *parser* recognizes contextually by their text: `fn`, `struct`, `var`, `if`, `else`, `while`, `for`, `ret`, `true`, `false`, `null`, `cast`, `va_start`, `va_arg`, `va_end`. This means these words are effectively reserved (using them as a variable/function/struct name will confuse the parser), even though the lexer treats them as ordinary identifiers.

## Built-in types

```
i8  i16  i32  i64
u8  u16  u32  u64
f32 f64
bool
rawptr
```

See [Types](/language/types/) for the full picture, including pointers, static arrays, slices and structs.

## Diagnostics

Compile errors are reported with a Rust-like caret diagnostic, pointing at the exact line/column, e.g.:

```
error: type inconnu : 'Fooo'
 --> hello.mw:4:14
  |
4 | var x: Fooo = 1;
  |        ^^^^
```

Marrow's own compiler diagnostics are currently written in **French** (the compiler's error messages are French strings, e.g. *"type inconnu"* = "unknown type", *"variable inconnue"* = "unknown variable"). Keep this in mind when reading raw compiler output — this documentation describes the underlying error in English, but what you'll see on your terminal is French.