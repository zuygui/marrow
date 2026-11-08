---
title: Methods and inheritance
description: impl blocks, self parameters, associated functions and single inheritance by composition.
sidebar:
  order: 10
---

## `impl` blocks

Functions that belong to a struct go in an `impl` block. A function whose first parameter is `self` is a **method**; the others are **associated functions**, called with `Type::name(...)`.

```marrow
struct Point {
    x: i64;
    y: i64;
}

impl Point {
    fn new(x: i64, y: i64) -> Point {
        ret Point { x: x, y: y };
    }

    fn sum(&self) -> i64 {
        ret self.x + self.y;
    }

    fn shift(&mut self, dx: i64) {
        self.x = self.x + dx;
    }
}

var p: Point = Point::new(3, 4);
p.shift(10);
var s: i64 = p.sum();      // 17
```

`self` can be written four ways, and each one is just a shorthand for a typed parameter:

| Written | Same as | Meaning |
| --- | --- | --- |
| `self` | `self: Point` | takes the value |
| `&self` | `self: &Point` | shared borrow |
| `&mut self` | `self: &mut Point` | mutable borrow |
| `^self` | `self: ^Point` | takes ownership (the receiver is moved) |

`p.shift(10)` borrows `p` the way the signature asks, and a receiver that is already a pointer (`&mut Point`, `^Point`) is passed as is. A type can have several `impl` blocks.

### Generic structs

A generic struct takes an `impl<T>` block. The parameters of the block are the ones of the struct, in the same order, and a method cannot add its own:

```marrow
struct Pair<T> { a: T; b: T; }

impl<T> Pair<T> {
    fn first(&self) -> T { ret self.a; }
    fn swap(&mut self) {
        var t: T = self.a;
        self.a = self.b;
        self.b = t;
    }
}
```

Each instance (`Pair<i64>`, `Pair<f64>`) gets its own copy of the methods, like any generic function.

Under the hood a method is an ordinary function named `Type__name` and `p.shift(10)` is `Point__shift(&mut p, 10)`; there is no run-time cost. A field with the same name as a method wins, so a function-pointer field is still callable.

## Single inheritance

`struct Child : Parent` makes the parent the first field of the child, called `base`.

```marrow
struct Animal {
    legs: i64;
    age: i64;
}

impl Animal {
    fn new(legs: i64, age: i64) -> Animal { ret Animal { legs: legs, age: age }; }
    fn describe(&self) -> i64 { ret self.legs * 100 + self.age; }
    fn birthday(&mut self) { self.age += 1; }
}

struct Dog : Animal {
    tricks: i64;
}

impl Dog {
    fn new(tricks: i64) -> Dog {
        ret Dog { base: Animal::new(4, 2), tricks: tricks };
    }

    // hides Animal::describe for dogs
    fn describe(&self) -> i64 { ret self.base.describe() * 10 + self.tricks; }
}

fn age_of(a: &Animal) -> i64 { ret a.age; }

var d: Dog = Dog::new(3);
d.birthday();            // inherited method
var n: i64 = d.age;      // inherited field, same as d.base.age
var a: i64 = age_of(&d); // &Dog converts to &Animal
```

- **Fields**: `d.age` is `d.base.age`, at any depth (`Puppy : Dog : Animal`).
- **Methods**: the compiler looks for the method on the child first, then on its parents. A child can **hide** a method by declaring one with the same name.
- **Conversions**: `&Child` and `&mut Child` convert to `&Parent` implicitly, because the parent sits at offset 0.
- **No virtual dispatch**: the method called depends on the static type. A function taking `&Animal` calls `Animal::describe` even when it is given a `Dog`.
- A struct has one parent; multiple inheritance is not supported.

## Limits

- A method of an `impl<T>` block cannot declare type parameters of its own.
- `impl` blocks for enums give associated functions only (`Color::code(c)`): enum values are plain integers, so `c.code()` is not available.
- `impl` and `self` are not reserved words; they are only special at the places above.
