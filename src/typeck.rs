//! Semantic analysis: name resolution and type checking.
//!
//! This pass runs between the parser and the code generator. It never stops at
//! the first problem: every error it finds is collected and returned, and the
//! code generator only runs when there is none. The special `RType::Error`
//! type is used after a failure so that one mistake does not trigger a cascade
//! of follow-up errors.
//!
//! Typing rules in short (see `implicit_ok` for the exact conversion table):
//!  - integer literals adopt the type they are used with (range-checked);
//!  - integers widen implicitly only when no value can be lost;
//!  - `*u8` and `rawptr` convert to and from every other pointer type;
//!  - everything else needs an explicit `cast(T) value`.

use std::collections::HashMap;

use crate::ast::*;
use crate::diagnostic::{suggest_similar, Diagnostic, SourceMap, Span};
use crate::types::{builtin_rtype, type_name, RType};

const BUILTINS: &[&str] = &["i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64", "bool", "rawptr"];
const KNOWN_DECORATORS: &[&str] = &["import", "extern", "export", "no_main"];

// ---------------------------------------------------------------------------
// Data collected about the program
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct FieldInfo {
    name: String,
    ty: RType,
    name_span: Span,
    ty_span: Span,
}

struct StructInfo {
    name_span: Span,
    fields: Vec<FieldInfo>,
}

#[derive(Clone)]
struct ParamInfo {
    name: String,
    ty: RType,
    span: Span,
}

#[derive(Clone)]
struct FnInfo {
    params: Vec<ParamInfo>,
    ret: RType,
    ret_span: Option<Span>,
    variadic: bool,
    name_span: Span,
    is_extern: bool,
}

#[derive(Clone)]
struct VarInfo {
    ty: RType,
    name_span: Span,
}

// ---------------------------------------------------------------------------
// Typed expression results
// ---------------------------------------------------------------------------

/// What we know about an expression's value beyond its type: untyped
/// literals can still take the type of the context they are used in.
#[derive(Clone, Debug, PartialEq)]
enum Lit {
    None,
    Int(i128),
    Float,
    Null,
    Str,
}

#[derive(Clone, Debug)]
struct TE {
    ty: RType,
    lit: Lit,
    /// The expression is a binary operation, assignment or cast: rewriting it
    /// inside a suggestion needs parentheses (`cast` swallows what follows it).
    compound: bool,
}

impl TE {
    fn new(ty: RType) -> TE {
        TE { ty, lit: Lit::None, compound: false }
    }
    fn lit(ty: RType, lit: Lit) -> TE {
        TE { ty, lit, compound: false }
    }
    fn error() -> TE {
        TE::new(RType::Error)
    }
}

/// Extra context for a conversion error.
struct Ctx {
    note: Option<(Span, String)>,
    /// The expression is an operand of a larger expression, so suggestions
    /// that rewrite it must be parenthesized.
    parens: bool,
}

impl Ctx {
    fn none() -> Ctx {
        Ctx { note: None, parens: false }
    }
    fn note(span: Span, text: impl Into<String>) -> Ctx {
        Ctx { note: Some((span, text.into())), parens: false }
    }
    fn operand() -> Ctx {
        Ctx { note: None, parens: true }
    }
}

fn found_desc(te: &TE) -> String {
    match te.lit {
        Lit::Int(_) => "integer".to_string(),
        Lit::Float => "float".to_string(),
        Lit::Null => "`null`".to_string(),
        _ => format!("`{}`", type_name(&te.ty)),
    }
}

// ---------------------------------------------------------------------------
// Conversion rules
// ---------------------------------------------------------------------------

fn widens(src: &RType, dst: &RType) -> bool {
    match (src.int_range(), dst.int_range()) {
        (Some((a, b)), Some((c, d))) => c <= a && b <= d,
        _ => false,
    }
}

fn int_to_float_lossless(src: &RType, dst: &RType) -> bool {
    let bits = src.int_width();
    match dst {
        RType::F64 => bits <= 32,
        RType::F32 => bits <= 16,
        _ => false,
    }
}

fn ptr_compat(a: &RType, b: &RType) -> bool {
    match (a, b) {
        (RType::RawPtr, RType::RawPtr) => true,
        (RType::RawPtr, RType::Pointer(_)) | (RType::Pointer(_), RType::RawPtr) => true,
        (RType::Pointer(x), RType::Pointer(y)) => {
            x == y || **x == RType::U8 || **y == RType::U8 || x.is_error() || y.is_error()
        }
        _ => false,
    }
}

/// Implicit conversion of a value of type `src` where `dst` is expected.
fn implicit_ok(src: &RType, dst: &RType) -> bool {
    if src == dst || src.is_error() || dst.is_error() {
        return true;
    }
    if src.is_int() && dst.is_int() {
        return widens(src, dst);
    }
    if *src == RType::Bool && dst.is_int() {
        return true;
    }
    if src.is_int() && dst.is_float() {
        return int_to_float_lossless(src, dst);
    }
    if *src == RType::F32 && *dst == RType::F64 {
        return true;
    }
    if src.is_pointer() && dst.is_pointer() {
        return ptr_compat(src, dst);
    }
    false
}

fn is_scalar(t: &RType) -> bool {
    t.is_integerish() || t.is_float()
}

/// Whether `cast(dst) value` is allowed for a value of type `src`.
fn cast_ok(src: &RType, dst: &RType) -> bool {
    if !is_scalar(src) || !is_scalar(dst) {
        return false;
    }
    !((src.is_float() && (dst.is_pointer() || dst.is_func())) || ((src.is_pointer() || src.is_func()) && dst.is_float()))
}

fn smallest_fitting_int(v: i128, signed_first: bool) -> Option<&'static str> {
    let signed: [(&str, i128, i128); 4] = [
        ("i8", i8::MIN as i128, i8::MAX as i128),
        ("i16", i16::MIN as i128, i16::MAX as i128),
        ("i32", i32::MIN as i128, i32::MAX as i128),
        ("i64", i64::MIN as i128, i64::MAX as i128),
    ];
    let unsigned: [(&str, i128, i128); 4] = [
        ("u8", 0, u8::MAX as i128),
        ("u16", 0, u16::MAX as i128),
        ("u32", 0, u32::MAX as i128),
        ("u64", 0, u64::MAX as i128),
    ];
    let order: [&[(&str, i128, i128); 4]; 2] = if signed_first { [&signed, &unsigned] } else { [&unsigned, &signed] };
    for set in order {
        for (name, lo, hi) in set.iter() {
            if *lo <= v && v <= *hi {
                return Some(name);
            }
        }
    }
    None
}

fn type_hint(name: &str) -> Option<&'static str> {
    Some(match name {
        "int" | "integer" | "short" => "Marrow has no `int` type: use a sized integer type such as `i32` or `i64`",
        "long" => "Marrow has no `long` type: use `i64`",
        "char" | "byte" | "uchar" => "Marrow has no `char` type: use `u8` for bytes and characters",
        "float" => "Marrow has no `float` type: use `f32`",
        "double" => "Marrow has no `double` type: use `f64`",
        "usize" | "size_t" | "uint" => "Marrow has no `usize` type: use `u64`",
        "isize" | "ssize_t" => "Marrow has no `isize` type: use `i64`",
        "string" | "str" | "String" => "Marrow has no string type: text is a pointer to bytes, `*u8`",
        "boolean" | "Bool" => "the boolean type is spelled `bool`",
        "void" => "`void` is not a type in Marrow: omit the return type of functions that return nothing",
        "any" | "voidptr" => "the untyped pointer type is `rawptr`",
        _ => return None,
    })
}

fn similar(name: &str, candidates: Vec<String>) -> Option<String> {
    suggest_similar(name, candidates.iter().map(|s| s.as_str())).map(|s| s.to_string())
}

fn const_truthy(e: &Expression) -> bool {
    match &e.kind {
        ExprKind::BoolLiteral(b) => *b,
        ExprKind::IntLiteral(v) => *v != 0,
        _ => false,
    }
}

fn stmt_terminates(s: &Statement) -> bool {
    match &s.kind {
        StmtKind::Return(_) => true,
        StmtKind::Block(b) => block_terminates(b),
        StmtKind::If(i) => if_terminates(i),
        // Only `ret` (or a `break` aimed at this very loop) can leave an infinite loop.
        StmtKind::While(w) => const_truthy(&w.cond) && !block_breaks(&w.body),
        StmtKind::For(f) => f.cond.as_ref().map_or(true, const_truthy) && !block_breaks(&f.body),
        StmtKind::Break | StmtKind::Continue => true,
        _ => false,
    }
}

/// Does `b` contain a `break` that leaves the loop owning `b`?
/// Nested loops own their own `break`s, so they are not searched.
fn block_breaks(b: &Block) -> bool {
    b.iter().any(stmt_breaks)
}

fn stmt_breaks(s: &Statement) -> bool {
    match &s.kind {
        StmtKind::Break => true,
        StmtKind::Block(b) => block_breaks(b),
        StmtKind::If(i) => if_breaks(i),
        _ => false,
    }
}

fn if_breaks(i: &IfStmt) -> bool {
    block_breaks(&i.then_block)
        || match &i.else_branch {
            None => false,
            Some(ElseBranch::Block(b)) => block_breaks(b),
            Some(ElseBranch::If(inner)) => if_breaks(inner),
        }
}

fn if_terminates(i: &IfStmt) -> bool {
    if !block_terminates(&i.then_block) {
        return false;
    }
    match &i.else_branch {
        None => false,
        Some(ElseBranch::Block(b)) => block_terminates(b),
        Some(ElseBranch::If(inner)) => if_terminates(inner),
    }
}

fn block_terminates(b: &Block) -> bool {
    b.iter().any(stmt_terminates)
}

fn is_place_kind(e: &Expression) -> bool {
    match &e.kind {
        ExprKind::Identifier(_) | ExprKind::Index { .. } | ExprKind::Member { .. } => true,
        ExprKind::Unary { op, .. } => op == "*",
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// The checker
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum ConstKind {
    Int,
    Float,
    Bool,
    Str,
    Null,
}

struct Checker<'a> {
    sm: &'a SourceMap,
    diags: Vec<Diagnostic>,
    structs: HashMap<String, StructInfo>,
    struct_order: Vec<String>,
    funcs: HashMap<String, FnInfo>,
    globals: HashMap<String, VarInfo>,
    scopes: Vec<HashMap<String, VarInfo>>,
    cur_ret: RType,
    cur_ret_span: Option<Span>,
    cur_fn_name_span: Span,
    cur_variadic: bool,
    loop_depth: u32,
}

pub fn check(program: &Program, sm: &SourceMap) -> Vec<Diagnostic> {
    let mut c = Checker {
        sm,
        diags: Vec::new(),
        structs: HashMap::new(),
        struct_order: Vec::new(),
        funcs: HashMap::new(),
        globals: HashMap::new(),
        scopes: Vec::new(),
        loop_depth: 0,
        cur_ret: RType::Void,
        cur_ret_span: None,
        cur_fn_name_span: Span::DUMMY,
        cur_variadic: false,
    };
    c.run(program);
    c.diags
}

impl<'a> Checker<'a> {
    fn err(&mut self, d: Diagnostic) {
        self.diags.push(d);
    }

    fn text(&self, span: Span) -> String {
        match self.sm.get(span.file) {
            Some(f) if !span.is_dummy() => f.text(span.start, span.end),
            _ => String::new(),
        }
    }

    // -- driver ------------------------------------------------------------

    fn run(&mut self, program: &Program) {
        // A: names of all structs, so that they can refer to each other.
        for item in &program.items {
            if let BindingValue::Struct(_) = &item.binding.value {
                let b = &item.binding;
                if let Some(prev) = self.structs.get(&b.name) {
                    let prev_span = prev.name_span;
                    self.err(dup_error(&b.name, b.name_span, prev_span));
                } else {
                    self.structs.insert(b.name.clone(), StructInfo { name_span: b.name_span, fields: Vec::new() });
                    self.struct_order.push(b.name.clone());
                }
            }
        }

        // B: struct fields.
        for item in &program.items {
            if let BindingValue::Struct(sd) = &item.binding.value {
                let b = &item.binding;
                if self.structs[&b.name].name_span != b.name_span {
                    continue; // duplicate definition, already reported
                }
                let mut fields: Vec<FieldInfo> = Vec::new();
                for f in &sd.fields {
                    let ty = self.resolve_type(&f.ty);
                    if let Some(prev) = fields.iter().find(|x| x.name == f.name) {
                        let d = Diagnostic::error(format!("field `{}` is already declared", f.name))
                            .code("E0308")
                            .primary(f.name_span, "field already declared")
                            .secondary(prev.name_span, "`".to_string() + &f.name + "` first declared here");
                        self.err(d);
                        continue;
                    }
                    fields.push(FieldInfo { name: f.name.clone(), ty, name_span: f.name_span, ty_span: f.ty.span });
                }
                self.structs.get_mut(&b.name).unwrap().fields = fields;
            }
        }
        self.check_recursive_structs();

        // C: function signatures and global variables.
        let n = program.items.len();
        let mut fn_sigs: Vec<Option<FnInfo>> = vec![None; n];
        for (i, item) in program.items.iter().enumerate() {
            self.check_decorators(item);
            let b = &item.binding;
            let is_extern = item.decorators.iter().any(|d| d.name == "extern");
            match &b.value {
                BindingValue::Function(fd) => {
                    if is_extern && fd.body.is_some() {
                        let ext = item.decorators.iter().find(|d| d.name == "extern").unwrap();
                        let d = Diagnostic::error(format!("`{}` is marked `@extern` but has a body", b.name))
                            .code("E0309")
                            .primary(fd.body_span.unwrap_or(b.name_span), "an external function cannot have a body")
                            .secondary(ext.span, "marked as external here")
                            .help("remove the body, or remove `@extern` to define the function here");
                        self.err(d);
                    }
                    if !is_extern && fd.body.is_none() {
                        let d = Diagnostic::error(format!("function `{}` has no body", b.name))
                            .code("E0310")
                            .primary(b.name_span, "this function has no body")
                            .help("add a body `{ ... }`, or mark it `@extern` if it is defined elsewhere");
                        self.err(d);
                    }
                    let sig = self.make_sig(&fd.params, fd.ret_type.as_ref(), fd.variadic, b.name_span, is_extern);
                    self.register_fn(&b.name, sig.clone());
                    fn_sigs[i] = Some(sig);
                }
                BindingValue::ExpressionFunction(efd) => {
                    if is_extern {
                        let ext = item.decorators.iter().find(|d| d.name == "extern").unwrap();
                        let d = Diagnostic::error(format!("`{}` is marked `@extern` but has a body", b.name))
                            .code("E0309")
                            .primary(efd.body.span, "an external function cannot have a body")
                            .secondary(ext.span, "marked as external here")
                            .help("use the bodiless form `fn name(...) -> Type;` instead");
                        self.err(d);
                    }
                    let sig = self.make_sig(&efd.params, efd.ret_type.as_ref(), efd.variadic, b.name_span, false);
                    self.register_fn(&b.name, sig.clone());
                    fn_sigs[i] = Some(sig);
                }
                BindingValue::Struct(_) => {
                    for d in item.decorators.iter().filter(|d| d.name == "extern" || d.name == "export") {
                        let diag = Diagnostic::error(format!("`@{}` cannot be applied to a struct", d.name))
                            .code("E0309")
                            .primary(d.span, "not applicable here")
                            .secondary(b.name_span, "this is a struct")
                            .note("`@extern` and `@export` only apply to functions");
                        self.err(diag);
                    }
                }
                BindingValue::Expr(e) => {
                    if let Some(ext) = item.decorators.iter().find(|d| d.name == "extern") {
                        let diag = Diagnostic::error("`@extern` cannot be applied to a global variable")
                            .code("E0309")
                            .primary(ext.span, "not applicable here")
                            .secondary(b.name_span, "this is a global variable")
                            .note("only functions can be `@extern`");
                        self.err(diag);
                    }
                    self.check_global(b, e);
                }
            }
        }

        // D: function bodies.
        for (i, item) in program.items.iter().enumerate() {
            let b = &item.binding;
            match (&b.value, &fn_sigs[i]) {
                (BindingValue::Function(fd), Some(sig)) => {
                    if let Some(body) = &fd.body {
                        self.check_fn_body(&b.name, sig, body, fd.body_span);
                    }
                }
                (BindingValue::ExpressionFunction(efd), Some(sig)) => {
                    self.check_expr_fn_body(&b.name, sig, &efd.body);
                }
                _ => {}
            }
        }
    }

    fn check_decorators(&mut self, item: &GlobalItem) {
        for d in &item.decorators {
            if !KNOWN_DECORATORS.contains(&d.name.as_str()) {
                let mut diag = Diagnostic::warning(format!("unknown decorator `@{}`", d.name))
                    .code("W0001")
                    .primary(d.name_span, "this decorator is ignored")
                    .note("the known decorators are `@import`, `@extern`, `@export` and `@no_main`");
                if let Some(s) = similar(&d.name, KNOWN_DECORATORS.iter().map(|s| s.to_string()).collect()) {
                    diag = diag.suggest("a decorator with a similar name exists", d.name_span, format!("@{}", s));
                }
                self.err(diag);
            }
            if d.name == "extern" {
                match &d.args {
                    None => {}
                    Some(args) if args.len() > 1 => {
                        let diag = Diagnostic::error("`@extern` takes at most one argument")
                            .code("E0309")
                            .primary(args[1].span, "unexpected extra argument")
                            .help("the only argument is the name of the C symbol: `@extern(\"puts\")`");
                        self.err(diag);
                    }
                    Some(args) => {
                        if let Some(a) = args.first() {
                            if !matches!(a.kind, ExprKind::StringLiteral(_)) {
                                let diag = Diagnostic::error("`@extern` expects a string literal")
                                    .code("E0309")
                                    .primary(a.span, "expected a string literal")
                                    .help("the argument is the name of the C symbol: `@extern(\"puts\")`");
                                self.err(diag);
                            }
                        }
                    }
                }
            }
        }
    }

    fn make_sig(&mut self, params: &[Parameter], ret_type: Option<&Type>, variadic: bool, name_span: Span, is_extern: bool) -> FnInfo {
        let mut out: Vec<ParamInfo> = Vec::new();
        for p in params {
            let ty = self.resolve_type(&p.ty);
            if let Some(prev) = out.iter().find(|x| x.name == p.name) {
                let d = Diagnostic::error(format!("identifier `{}` is bound more than once in this parameter list", p.name))
                    .code("E0308")
                    .primary(p.name_span, "used as a parameter more than once")
                    .secondary(prev.span, "first use of `".to_string() + &p.name + "`");
                self.err(d);
                continue;
            }
            out.push(ParamInfo { name: p.name.clone(), ty, span: p.span() });
        }
        let (ret, ret_span) = match ret_type {
            Some(t) => (self.resolve_type(t), Some(t.span)),
            None => (RType::Void, None),
        };
        FnInfo { params: out, ret, ret_span, variadic, name_span, is_extern }
    }

    fn register_fn(&mut self, name: &str, sig: FnInfo) {
        if let Some(prev) = self.funcs.get(name) {
            // Several files may declare the same C function; that is fine
            // as long as they agree.
            let same = prev.is_extern
                && sig.is_extern
                && prev.ret == sig.ret
                && prev.variadic == sig.variadic
                && prev.params.len() == sig.params.len()
                && prev.params.iter().zip(sig.params.iter()).all(|(a, b)| a.ty == b.ty);
            if same {
                return;
            }
            let prev_span = prev.name_span;
            let mut d = dup_error(name, sig.name_span, prev_span);
            if prev.is_extern && sig.is_extern {
                d = d.note("both declarations are `@extern` but their signatures differ");
            }
            self.err(d);
            return;
        }
        if let Some(prev) = self.globals.get(name) {
            let prev_span = prev.name_span;
            self.err(dup_error(name, sig.name_span, prev_span));
            return;
        }
        self.funcs.insert(name.to_string(), sig);
    }

    fn check_recursive_structs(&mut self) {
        let mut reported: Vec<String> = Vec::new();
        let names = self.struct_order.clone();
        for start in &names {
            if reported.contains(start) {
                continue;
            }
            let mut path: Vec<(String, Span)> = Vec::new();
            let mut visiting: Vec<String> = Vec::new();
            if self.find_cycle(start, start, &mut visiting, &mut path) {
                let first_span = path[0].1;
                let cycle: Vec<String> = path.iter().map(|(n, _)| n.clone()).collect();
                let mut d = Diagnostic::error(format!("recursive type `{}` has infinite size", start))
                    .code("E0311")
                    .primary(first_span, "recursive without indirection");
                if cycle.len() > 1 {
                    d = d.note(format!("`{}` contains itself through: {}", start, cycle.join(" -> ")));
                }
                d = d.help("use a pointer to break the cycle, e.g. `*".to_string() + &type_text_of(&self.text(first_span)) + "`");
                self.err(d);
                for n in cycle {
                    reported.push(n);
                }
            }
        }
    }

    /// Depth-first search for a by-value containment path from `cur` back to
    /// `target`. `path` receives, for every struct on the way, the span of the
    /// field type that leads to the next one.
    fn find_cycle(&self, cur: &str, target: &str, visiting: &mut Vec<String>, path: &mut Vec<(String, Span)>) -> bool {
        if visiting.iter().any(|v| v == cur) {
            return false;
        }
        visiting.push(cur.to_string());
        let info = match self.structs.get(cur) {
            Some(i) => i,
            None => return false,
        };
        for f in &info.fields {
            let mut contained: Vec<String> = Vec::new();
            collect_by_value(&f.ty, &mut contained);
            for c in contained {
                path.push((cur.to_string(), f.ty_span));
                if c == target {
                    return true;
                }
                if self.find_cycle(&c, target, visiting, path) {
                    return true;
                }
                path.pop();
            }
        }
        false
    }

    // -- types -------------------------------------------------------------

    fn resolve_type(&mut self, t: &Type) -> RType {
        match &t.kind {
            TypeKind::Builtin(s) => builtin_rtype(s).unwrap_or(RType::Error),
            TypeKind::Custom(name) => {
                if self.structs.contains_key(name) {
                    RType::Struct(name.clone())
                } else {
                    self.unknown_type(name, t.span);
                    RType::Error
                }
            }
            TypeKind::StaticArray(size, inner) => {
                let n = self.const_eval_size(size);
                let inner_t = self.resolve_type(inner);
                match n {
                    Some(n) if !inner_t.is_error() => RType::Array(n, Box::new(inner_t)),
                    _ => RType::Error,
                }
            }
            TypeKind::Pointer(inner) => {
                let i = self.resolve_type(inner);
                if i.is_error() {
                    RType::Error
                } else {
                    RType::Pointer(Box::new(i))
                }
            }
            TypeKind::Slice(inner) => {
                let i = self.resolve_type(inner);
                if i.is_error() {
                    RType::Error
                } else {
                    RType::Slice(Box::new(i))
                }
            }
            TypeKind::Func(params, ret) => {
                let ps: Vec<RType> = params.iter().map(|p| self.resolve_type(p)).collect();
                let r = match ret {
                    Some(r) => self.resolve_type(r),
                    None => RType::Void,
                };
                if ps.iter().any(|p| p.is_error()) || r.is_error() {
                    RType::Error
                } else {
                    RType::Func(ps, Box::new(r))
                }
            }
        }
    }

    fn unknown_type(&mut self, name: &str, span: Span) {
        let mut d = Diagnostic::error(format!("cannot find type `{}` in this scope", name))
            .code("E0401")
            .primary(span, "not found in this scope");
        if let Some(h) = type_hint(name) {
            d = d.help(h);
        } else {
            let mut cands: Vec<String> = self.structs.keys().cloned().collect();
            cands.extend(BUILTINS.iter().map(|s| s.to_string()));
            if let Some(s) = similar(name, cands) {
                d = d.suggest("a type with a similar name exists", span, s);
            } else if self.funcs.contains_key(name) || self.globals.contains_key(name) {
                d = d.note(format!("`{}` is a value, not a type", name));
            }
        }
        self.err(d);
    }

    /// Evaluates the size expression of a static array type.
    fn const_eval_size(&mut self, e: &Expression) -> Option<u64> {
        match &e.kind {
            ExprKind::IntLiteral(v) => {
                if *v < 0 || *v > u64::MAX as i128 {
                    self.err(Diagnostic::error("array size is out of range").code("E0312").primary(e.span, "not a valid array size"));
                    None
                } else {
                    Some(*v as u64)
                }
            }
            ExprKind::Unary { op, .. } if op == "-" => {
                self.err(Diagnostic::error("array size cannot be negative").code("E0312").primary(e.span, "negative size"));
                None
            }
            ExprKind::Binary { op, left, right, .. } => {
                let l = self.const_eval_size(left);
                let r = self.const_eval_size(right);
                let (l, r) = (l?, r?);
                let res = match op.as_str() {
                    "+" => l.checked_add(r),
                    "-" => l.checked_sub(r),
                    "*" => l.checked_mul(r),
                    "/" => {
                        if r == 0 {
                            self.err(Diagnostic::error("division by zero in an array size").code("E0312").primary(e.span, "attempt to divide by zero"));
                            return None;
                        }
                        Some(l / r)
                    }
                    _ => {
                        self.err(
                            Diagnostic::error(format!("operator `{}` is not supported in array sizes", op))
                                .code("E0312")
                                .primary(e.span, "not supported here")
                                .note("array sizes support integer literals and `+`, `-`, `*`, `/`"),
                        );
                        return None;
                    }
                };
                if res.is_none() {
                    self.err(Diagnostic::error("array size is out of range").code("E0312").primary(e.span, "this overflows or is negative"));
                }
                res
            }
            ExprKind::Identifier(name) => {
                self.err(
                    Diagnostic::error("array size must be a constant integer expression")
                        .code("E0312")
                        .primary(e.span, format!("`{}` is not a constant integer literal", name))
                        .help("named constants cannot be used in array sizes yet: write the number directly"),
                );
                None
            }
            _ => {
                self.err(
                    Diagnostic::error("array size must be a constant integer expression")
                        .code("E0312")
                        .primary(e.span, "not a constant"),
                );
                None
            }
        }
    }

    // -- globals -----------------------------------------------------------

    fn check_global(&mut self, b: &BindingDecl, init: &Expression) {
        let declared = b.ty.as_ref().map(|t| (self.resolve_type(t), t.span));
        let te = if self.const_kind(init).is_some() { Some(self.infer(init)) } else { None };

        let ty = match (&declared, &te) {
            (Some((dt, tspan)), Some(te)) => {
                self.coerce(te, dt, init.span, Ctx::note(*tspan, "expected due to this type"));
                if dt.is_aggregate() {
                    let d = Diagnostic::error("global variables of aggregate type are not supported")
                        .code("E0312")
                        .primary(*tspan, format!("`{}` is a struct, array or slice type", type_name(dt)))
                        .note("global variables must be numbers, booleans or pointers");
                    self.err(d);
                } else if (matches!(dt, RType::RawPtr) || matches!(dt, RType::Pointer(_)))
                    && matches!(te.lit, Lit::Str)
                    && *dt != RType::Pointer(Box::new(RType::U8))
                {
                    let d = Diagnostic::error("a string can only initialize a global of type `*u8`")
                        .code("E0312")
                        .primary(init.span, format!("this string cannot initialize `{}`", type_name(dt)))
                        .secondary(*tspan, "declared with this type");
                    self.err(d);
                }
                dt.clone()
            }
            (Some((dt, _)), None) => dt.clone(),
            (None, Some(te)) => self.default_type(te, init.span),
            (None, None) => RType::Error,
        };

        if let Some(prev) = self.globals.get(&b.name) {
            let prev_span = prev.name_span;
            self.err(dup_error(&b.name, b.name_span, prev_span));
            return;
        }
        if let Some(prev) = self.funcs.get(&b.name) {
            let prev_span = prev.name_span;
            self.err(dup_error(&b.name, b.name_span, prev_span));
            return;
        }
        self.globals.insert(b.name.clone(), VarInfo { ty, name_span: b.name_span });
    }

    /// Checks that a global initializer can be evaluated at compile time.
    fn const_kind(&mut self, e: &Expression) -> Option<ConstKind> {
        match &e.kind {
            ExprKind::IntLiteral(_) | ExprKind::CharLiteral(_) => Some(ConstKind::Int),
            ExprKind::FloatLiteral(_) => Some(ConstKind::Float),
            ExprKind::BoolLiteral(_) => Some(ConstKind::Bool),
            ExprKind::StringLiteral(_) => Some(ConstKind::Str),
            ExprKind::Null => Some(ConstKind::Null),
            ExprKind::Unary { op, expr } if op == "~" => {
                let k = self.const_kind(expr)?;
                if k == ConstKind::Int {
                    Some(k)
                } else {
                    self.err(Diagnostic::error("`~` needs an integer constant").code("E0312").primary(e.span, "not an integer"));
                    None
                }
            }
            ExprKind::Unary { op, expr } if op == "-" => {
                let k = self.const_kind(expr)?;
                if k == ConstKind::Int || k == ConstKind::Float {
                    Some(k)
                } else {
                    self.err(Diagnostic::error("cannot negate a non-numeric constant").code("E0312").primary(e.span, "not a number"));
                    None
                }
            }
            ExprKind::Binary { op, left, right, op_span } => {
                let l = self.const_kind(left);
                let r = self.const_kind(right);
                let (l, r) = (l?, r?);
                if !matches!(op.as_str(), "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^" | "<<" | ">>") {
                    self.err(
                        Diagnostic::error(format!("operator `{}` is not supported in global initializers", op))
                            .code("E0312")
                            .primary(*op_span, "not supported here")
                            .note("global initializers support literals and `+ - * / % & | ^ << >>`"),
                    );
                    return None;
                }
                match (l, r) {
                    (ConstKind::Int, ConstKind::Int) => {
                        if op == "/" || op == "%" {
                            if let ExprKind::IntLiteral(0) = right.kind {
                                self.err(Diagnostic::error("division by zero in a global initializer").code("E0312").primary(right.span, "attempt to divide by zero"));
                                return None;
                            }
                        }
                        Some(ConstKind::Int)
                    }
                    (ConstKind::Float, ConstKind::Float) if matches!(op.as_str(), "+" | "-" | "*" | "/") => Some(ConstKind::Float),
                    _ => {
                        self.err(
                            Diagnostic::error("mismatched operands in a global initializer")
                                .code("E0312")
                                .primary(e.span, "both operands must be integers, or both must be floats")
                                .help("write a float literal such as `1.0` instead of `1`"),
                        );
                        None
                    }
                }
            }
            ExprKind::Identifier(name) => {
                self.err(
                    Diagnostic::error("global initializers must be constant")
                        .code("E0312")
                        .primary(e.span, format!("`{}` cannot be used here", name))
                        .note("global variables cannot refer to other variables or functions yet"),
                );
                None
            }
            _ => {
                self.err(Diagnostic::error("global initializers must be constant").code("E0312").primary(e.span, "this is not a compile-time constant").note(
                    "global initializers support literals and `+`, `-`, `*`, `/`",
                ));
                None
            }
        }
    }

    // -- scopes ------------------------------------------------------------

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare_local(&mut self, name: &str, ty: RType, name_span: Span) {
        if let Some(prev) = self.scopes.last().and_then(|s| s.get(name)) {
            let prev_span = prev.name_span;
            let d = Diagnostic::warning(format!("variable `{}` is already declared in this scope", name))
                .code("W0004")
                .primary(name_span, "this declaration shadows the previous one")
                .secondary(prev_span, "previous declaration")
                .help("remove the first declaration or use another name");
            self.err(d);
        }
        if let Some(s) = self.scopes.last_mut() {
            s.insert(name.to_string(), VarInfo { ty, name_span });
        }
    }

    fn lookup_var(&self, name: &str) -> Option<VarInfo> {
        for s in self.scopes.iter().rev() {
            if let Some(v) = s.get(name) {
                return Some(v.clone());
            }
        }
        self.globals.get(name).cloned()
    }

    // -- function bodies ---------------------------------------------------

    fn enter_fn(&mut self, sig: &FnInfo) {
        self.cur_ret = sig.ret.clone();
        self.cur_ret_span = sig.ret_span;
        self.cur_fn_name_span = sig.name_span;
        self.cur_variadic = sig.variadic;
        self.scopes = vec![HashMap::new()];
        for p in &sig.params {
            self.scopes[0].insert(p.name.clone(), VarInfo { ty: p.ty.clone(), name_span: p.span });
        }
    }

    fn check_fn_body(&mut self, name: &str, sig: &FnInfo, body: &Block, body_span: Option<Span>) {
        self.enter_fn(sig);
        let terminates = self.check_block(body);
        if !terminates && sig.ret != RType::Void && !sig.ret.is_error() {
            let end = body_span.map(|s| Span::new(s.file, s.end.saturating_sub(1), s.end)).unwrap_or(sig.name_span);
            let mut d = Diagnostic::error(format!("function `{}` does not return a value on all code paths", name))
                .code("E0410")
                .primary(end, format!("reaches the end of the function without returning `{}`", type_name(&sig.ret)));
            if let Some(rs) = sig.ret_span {
                d = d.secondary(rs, format!("expected `{}` because of this return type", type_name(&sig.ret)));
            }
            d = d.help("add a `ret` with a value at the end of the function, or remove the return type if the function returns nothing");
            self.err(d);
        }
        self.scopes.clear();
    }

    fn check_expr_fn_body(&mut self, name: &str, sig: &FnInfo, body: &Expression) {
        self.enter_fn(sig);
        let te = self.infer(body);
        if sig.ret == RType::Void {
            if !te.ty.is_error() {
                let mut d = Diagnostic::error(format!("function `{}` has an expression body but no return type", name))
                    .code("E0410")
                    .primary(body.span, "this expression is returned")
                    .secondary(sig.name_span, "this function returns nothing");
                if te.ty != RType::Void {
                    d = d.help(format!("add a return type: `-> {}`", type_name(&self.default_ty_quiet(&te))));
                } else {
                    d = d.help("use a block body `{ ... }` for functions that return nothing");
                }
                self.err(d);
            }
        } else {
            let ctx = match sig.ret_span {
                Some(rs) => Ctx::note(rs, format!("expected `{}` because of this return type", type_name(&sig.ret))),
                None => Ctx::none(),
            };
            self.coerce(&te, &sig.ret.clone(), body.span, ctx);
        }
        self.scopes.clear();
    }

    // -- statements --------------------------------------------------------

    fn check_block(&mut self, b: &Block) -> bool {
        self.push_scope();
        let t = self.check_stmts(b);
        self.pop_scope();
        t
    }

    fn check_stmts(&mut self, b: &Block) -> bool {
        let mut terminated_at: Option<Span> = None;
        let mut warned = false;
        for s in b {
            if let Some(prev) = terminated_at {
                if !warned {
                    warned = true;
                    let d = Diagnostic::warning("unreachable statement")
                        .code("W0002")
                        .primary(s.span, "this code is never executed")
                        .secondary(prev, "any code following this statement is unreachable");
                    self.err(d);
                }
            }
            self.check_stmt(s);
            if terminated_at.is_none() && stmt_terminates(s) {
                terminated_at = Some(s.span);
            }
        }
        terminated_at.is_some()
    }

    fn check_stmt(&mut self, s: &Statement) {
        match &s.kind {
            StmtKind::Block(b) => {
                self.check_block(b);
            }
            StmtKind::LocalVarDecl(decl) => self.check_local_decl(decl),
            StmtKind::If(i) => self.check_if(i),
            StmtKind::While(w) => {
                self.check_cond(&w.cond);
                self.loop_depth += 1;
                self.check_block(&w.body);
                self.loop_depth -= 1;
            }
            StmtKind::Break | StmtKind::Continue => {
                if self.loop_depth == 0 {
                    let word = if matches!(s.kind, StmtKind::Break) { "break" } else { "continue" };
                    let d = Diagnostic::error(format!("`{}` outside of a loop", word))
                        .code("E0416")
                        .primary(s.span, format!("`{}` is only allowed inside `while` or `for`", word));
                    self.err(d);
                }
            }
            StmtKind::For(f) => {
                self.push_scope();
                if let Some(init) = &f.init {
                    self.check_local_decl(init);
                }
                if let Some(c) = &f.cond {
                    self.check_cond(c);
                }
                if let Some(p) = &f.post {
                    self.infer(p);
                }
                self.loop_depth += 1;
                self.check_block(&f.body);
                self.loop_depth -= 1;
                self.pop_scope();
            }
            StmtKind::Return(e) => self.check_return(e.as_ref(), s.span),
            StmtKind::Expr(e) => {
                self.infer(e);
                self.warn_unused(e);
            }
        }
    }

    fn warn_unused(&mut self, e: &Expression) {
        let what = match &e.kind {
            ExprKind::Binary { op, .. } => match op.as_str() {
                "==" | "!=" | "<" | "<=" | ">" | ">=" => "comparison",
                "&&" | "||" => "logical operation",
                _ => "arithmetic operation",
            },
            ExprKind::Unary { op, .. } if op != "*" => "unary operation",
            ExprKind::Identifier(_) | ExprKind::IntLiteral(_) | ExprKind::FloatLiteral(_) | ExprKind::StringLiteral(_) | ExprKind::BoolLiteral(_) => "value",
            _ => return,
        };
        let mut d = Diagnostic::warning(format!("unused {} that must be used", what))
            .code("W0005")
            .primary(e.span, "the result is not used");
        if what == "comparison" {
            d = d.help("did you forget an `if` or `while`?");
        }
        self.err(d);
    }

    fn check_if(&mut self, i: &IfStmt) {
        self.check_cond(&i.cond);
        self.check_block(&i.then_block);
        match &i.else_branch {
            None => {}
            Some(ElseBranch::Block(b)) => {
                self.check_block(b);
            }
            Some(ElseBranch::If(inner)) => self.check_if(inner),
        }
    }

    fn check_cond(&mut self, e: &Expression) {
        if let ExprKind::Assign { op, op_span, .. } = &e.kind {
            if op == "=" {
                let d = Diagnostic::warning("assignment used as a condition")
                    .code("W0003")
                    .primary(*op_span, "this is an assignment, not a comparison")
                    .suggest("if you meant to compare the values, use `==`", *op_span, "==");
                self.err(d);
            }
        }
        let te = self.infer(e);
        self.require_scalar(&te, e.span, "condition");
    }

    /// Conditions and logical operands must be numbers, booleans or pointers.
    fn require_scalar(&mut self, te: &TE, span: Span, what: &str) -> bool {
        if te.ty.is_error() {
            return false;
        }
        if te.ty == RType::Void {
            self.void_value_error(span);
            return false;
        }
        if te.ty.is_aggregate() {
            let d = Diagnostic::error(format!("mismatched types: this {} must be a scalar", what))
                .code("E0411")
                .primary(span, format!("expected `bool`, found `{}`", type_name(&te.ty)))
                .note("structs, arrays and slices cannot be used as conditions");
            self.err(d);
            return false;
        }
        true
    }

    fn void_value_error(&mut self, span: Span) {
        let d = Diagnostic::error("this expression has no value")
            .code("E0413")
            .primary(span, "this call returns nothing (type `void`)")
            .help("a function without a return type cannot be used as a value");
        self.err(d);
    }

    fn check_local_decl(&mut self, decl: &LocalVarDecl) {
        match decl {
            LocalVarDecl::Mutable { ty, name, name_span, value } => {
                let declared = ty.as_ref().map(|t| (self.resolve_type(t), t.span));
                if matches!(value.kind, ExprKind::Zeroed) {
                    if let Some((dt, _)) = declared {
                        self.declare_local(name, dt, *name_span);
                    }
                    return;
                }
                let te = self.infer(value);
                let var_ty = match declared {
                    Some((dt, tspan)) => {
                        self.coerce(&te, &dt, value.span, Ctx::note(tspan, "expected due to this type"));
                        dt
                    }
                    None => self.default_type(&te, value.span),
                };
                self.declare_local(name, var_ty, *name_span);
            }
            LocalVarDecl::Constant { name, .. } => {
                let d = Diagnostic::error(format!("local declaration of `{}` is not supported", name))
                    .code("E0309")
                    .primary(self.cur_fn_name_span, "inside this function");
                self.err(d);
            }
        }
    }

    fn check_return(&mut self, e: Option<&Expression>, span: Span) {
        let ret = self.cur_ret.clone();
        let ret_span = self.cur_ret_span;
        match e {
            None => {
                if ret != RType::Void && !ret.is_error() {
                    let mut d = Diagnostic::error("`ret` without a value in a function that returns one")
                        .code("E0410")
                        .primary(span, format!("expected a value of type `{}`", type_name(&ret)));
                    if let Some(rs) = ret_span {
                        d = d.secondary(rs, format!("expected `{}` because of this return type", type_name(&ret)));
                    }
                    self.err(d);
                }
            }
            Some(expr) => {
                let te = self.infer(expr);
                if ret == RType::Void {
                    if !te.ty.is_error() {
                        let d = Diagnostic::error("unexpected return value in a function that returns nothing")
                            .code("E0410")
                            .primary(expr.span, "this value is returned")
                            .secondary(self.cur_fn_name_span, "this function has no return type")
                            .help("remove the value, or add a return type to the function");
                        self.err(d);
                    }
                } else {
                    let ctx = match ret_span {
                        Some(rs) => Ctx::note(rs, format!("expected `{}` because of this return type", type_name(&ret))),
                        None => Ctx::none(),
                    };
                    self.coerce(&te, &ret, expr.span, ctx);
                }
            }
        }
    }

    // -- conversions -------------------------------------------------------

    /// The type a value gets when it initializes a variable without a type
    /// annotation.
    fn default_type(&mut self, te: &TE, span: Span) -> RType {
        match te.lit {
            Lit::Int(v) => {
                if v < i64::MIN as i128 || v > i64::MAX as i128 {
                    self.range_error(v, &RType::I64, span, true);
                }
                RType::I64
            }
            Lit::Float => RType::F64,
            Lit::Null => RType::RawPtr,
            _ => {
                if te.ty == RType::Void {
                    self.void_value_error(span);
                    return RType::Error;
                }
                te.ty.clone()
            }
        }
    }

    fn default_ty_quiet(&self, te: &TE) -> RType {
        match te.lit {
            Lit::Int(_) => RType::I64,
            Lit::Float => RType::F64,
            Lit::Null => RType::RawPtr,
            _ => te.ty.clone(),
        }
    }

    fn range_error(&mut self, v: i128, target: &RType, span: Span, defaulted: bool) {
        let (lo, hi) = target.int_range().unwrap();
        let mut d = Diagnostic::error(format!("literal out of range for `{}`", type_name(target)))
            .code("E0409")
            .primary(span, format!("the literal `{}` does not fit into the type `{}` whose range is `{}..={}`", v, type_name(target), lo, hi));
        if defaulted {
            d = d.note("untyped integer literals default to `i64`; add a type annotation such as `: u64`");
        } else if v < 0 && !target.is_signed_int() {
            d = d.note("unsigned types cannot hold negative values");
            d = d.help("use a signed type such as `i32` or `i64` instead");
        } else if let Some(t) = smallest_fitting_int(v, target.is_signed_int()) {
            d = d.help(format!("consider using the type `{}` instead", t));
        }
        d = d.help(format!("to reinterpret the bits on purpose, use an explicit cast: `cast({}) {}`", type_name(target), v));
        self.err(d);
    }

    /// Checks that `te` can be used where `target` is expected.
    fn coerce(&mut self, te: &TE, target: &RType, span: Span, ctx: Ctx) -> bool {
        let src = &te.ty;
        if src.is_error() || target.is_error() {
            return true;
        }
        if *src == RType::Void {
            self.void_value_error(span);
            return false;
        }
        match &te.lit {
            Lit::Int(v) => {
                if target.is_int() {
                    let (lo, hi) = target.int_range().unwrap();
                    if *v < lo || *v > hi {
                        self.range_error(*v, target, span, false);
                        return false;
                    }
                    return true;
                }
                if target.is_float() {
                    return true;
                }
                if *target == RType::Bool && (*v == 0 || *v == 1) {
                    return true;
                }
                if (target.is_pointer() || target.is_func()) && *v == 0 {
                    return true;
                }
            }
            Lit::Float => {
                if target.is_float() {
                    return true;
                }
            }
            Lit::Null => {
                if target.is_pointer() || target.is_func() {
                    return true;
                }
            }
            _ => {}
        }
        if !matches!(te.lit, Lit::Int(_) | Lit::Float | Lit::Null) && implicit_ok(src, target) {
            return true;
        }
        self.mismatch(te, target, span, ctx);
        false
    }

    fn mismatch(&mut self, te: &TE, target: &RType, span: Span, ctx: Ctx) {
        let mut d = Diagnostic::error("mismatched types")
            .code("E0402")
            .primary(span, format!("expected `{}`, found {}", type_name(target), found_desc(te)));
        if let Some((s, t)) = ctx.note {
            d = d.secondary(s, t);
        }
        if te.lit == Lit::Str {
            d = d.note("string literals have type `*u8`");
        }
        let text = self.text(span);
        let single_line = !text.is_empty() && !text.contains('\n');
        let operand = if te.compound && !fully_parenthesized(&text) { format!("({})", text) } else { text.clone() };
        let wrap = |s: String| if ctx.parens { format!("({})", s) } else { s };
        let src = &te.ty;
        match &te.lit {
            Lit::Int(v) if *target == RType::Bool => {
                d = d.note(format!("`{}` is not a boolean: use `true` or `false`", v));
            }
            Lit::Int(_) if target.is_pointer() => {
                d = d.note("only the literal `0` converts to a pointer implicitly");
                if single_line {
                    d = d.suggest("to build a pointer from a number, use an explicit cast", span, wrap(format!("cast({}) {}", type_name(target), operand)));
                }
            }
            Lit::Str => {}
            _ => {
                if *target == RType::Bool && (src.is_int() || src.is_pointer()) && single_line {
                    d = d.suggest("to test for a non-zero value, compare explicitly", span, wrap(format!("{} != 0", operand)));
                } else if src.is_float() && target.is_int() && single_line {
                    d = d.suggest("floats do not convert to integers implicitly (the fraction is dropped); use an explicit cast", span, wrap(format!("cast({}) {}", type_name(target), operand)));
                } else if is_scalar(src) && is_scalar(target) && cast_ok(src, target) && single_line {
                    d = d.suggest("you can convert the value with an explicit cast", span, wrap(format!("cast({}) {}", type_name(target), operand)));
                } else if let (RType::Array(_, a), RType::Slice(b)) = (src, target) {
                    if a == b && single_line {
                        d = d.suggest("an array does not convert to a slice implicitly; slice it first", span, format!("{}[..]", operand));
                    }
                } else if src.is_pointer() && target.is_pointer() {
                    d = d.note("only `*u8` and `rawptr` convert implicitly to other pointer types");
                    if single_line {
                        d = d.suggest("use an explicit cast", span, wrap(format!("cast({}) {}", type_name(target), operand)));
                    }
                }
            }
        }
        self.err(d);
    }

    // -- expressions -------------------------------------------------------

    fn infer(&mut self, e: &Expression) -> TE {
        let mut te = self.infer_inner(e);
        te.compound = matches!(e.kind, ExprKind::Binary { .. } | ExprKind::Assign { .. } | ExprKind::Cast { .. });
        te
    }

    fn infer_inner(&mut self, e: &Expression) -> TE {
        match &e.kind {
            ExprKind::Identifier(name) => self.infer_ident(name, e.span),
            ExprKind::IntLiteral(v) => TE::lit(RType::I64, Lit::Int(*v)),
            ExprKind::FloatLiteral(_) => TE::lit(RType::F64, Lit::Float),
            ExprKind::StringLiteral(_) => TE::lit(RType::Pointer(Box::new(RType::U8)), Lit::Str),
            ExprKind::CharLiteral(c) => {
                if (*c as u32) > 0xFF {
                    let d = Diagnostic::error("character literal out of range for `u8`")
                        .code("E0409")
                        .primary(e.span, format!("`{}` is U+{:04X}, which does not fit in one byte", c, *c as u32))
                        .note("character literals have type `u8`");
                    self.err(d);
                    return TE::error();
                }
                TE::new(RType::U8)
            }
            ExprKind::BoolLiteral(_) => TE::new(RType::Bool),
            ExprKind::Null => TE::lit(RType::RawPtr, Lit::Null),
            ExprKind::StructInit { name, name_span, fields } => self.infer_struct_init(name, *name_span, fields, e.span),
            ExprKind::Cast { ty, expr } => self.infer_cast(ty, expr, e.span),
            ExprKind::Unary { op, expr } => self.infer_unary(op, expr, e.span),
            ExprKind::Binary { op, op_span, left, right } => self.infer_binary(op, *op_span, left, right, e.span),
            ExprKind::Assign { op, op_span, target, value } => self.infer_assign(op, *op_span, target, value, e.span),
            ExprKind::Call { callee, args } => self.infer_call(callee, args, e.span),
            ExprKind::Index { base, index } => self.infer_index(base, index, e.span),
            ExprKind::Member { base, member, member_span } => self.infer_member(base, member, *member_span, e.span),
            ExprKind::Slice { base, start, end } => self.infer_slice(base, start.as_deref(), end.as_deref(), e.span),
            ExprKind::SizeOf(t) => {
                let rt = self.resolve_type(t);
                if rt == RType::Void {
                    self.err(Diagnostic::error("`sizeof` of `void`").code("E0417").primary(t.span, "`void` has no size"));
                    return TE::error();
                }
                TE::new(RType::U64)
            }
            ExprKind::Zeroed => TE::error(),
            ExprKind::VaStart => {
                if !self.cur_variadic {
                    let d = Diagnostic::error("`va_start` can only be used in a variadic function")
                        .code("E0415")
                        .primary(e.span, "this function is not variadic")
                        .secondary(self.cur_fn_name_span, "declared here")
                        .help("add `...` at the end of the parameter list");
                    self.err(d);
                }
                TE::new(RType::RawPtr)
            }
            ExprKind::VaArg { list, ty } => {
                let lt = self.infer(list);
                self.coerce(&lt, &RType::RawPtr, list.span, Ctx::none());
                let t = self.resolve_type(ty);
                if t.is_aggregate() {
                    let d = Diagnostic::error("`va_arg` can only read scalar types")
                        .code("E0415")
                        .primary(ty.span, format!("`{}` is a struct, array or slice", type_name(&t)))
                        .help("pass a pointer instead and read through it");
                    self.err(d);
                    return TE::error();
                }
                if matches!(t, RType::I8 | RType::I16 | RType::U8 | RType::U16 | RType::Bool | RType::F32) {
                    let want = match t {
                        RType::F32 => "f64",
                        RType::U8 | RType::U16 => "u32",
                        _ => "i32",
                    };
                    let d = Diagnostic::error(format!("`va_arg` cannot read `{}`", type_name(&t)))
                        .code("E0415")
                        .primary(ty.span, "this type is promoted when passed as a variadic argument")
                        .note("variadic arguments undergo the C default promotions")
                        .suggest(format!("read it as `{}` instead", want), ty.span, want);
                    self.err(d);
                    return TE::error();
                }
                TE::new(t)
            }
            ExprKind::VaEnd(list) => {
                let lt = self.infer(list);
                self.coerce(&lt, &RType::RawPtr, list.span, Ctx::none());
                TE::new(RType::Void)
            }
        }
    }

    fn infer_ident(&mut self, name: &str, span: Span) -> TE {
        if let Some(v) = self.lookup_var(name) {
            return TE::new(v.ty);
        }
        if let Some(f) = self.funcs.get(name) {
            let fspan = f.name_span;
            if f.variadic {
                let d = Diagnostic::error(format!("variadic function `{}` cannot be used as a value", name))
                    .code("E0305")
                    .primary(span, "function pointers cannot point to variadic functions")
                    .secondary(fspan, "function defined here");
                self.err(d);
                return TE::error();
            }
            let params: Vec<RType> = f.params.iter().map(|p| p.ty.clone()).collect();
            return TE::new(RType::Func(params, Box::new(f.ret.clone())));
        }
        if self.structs.contains_key(name) {
            let d = Diagnostic::error(format!("expected a value, found struct `{}`", name))
                .code("E0305")
                .primary(span, "this is a type, not a value")
                .help(format!("to create a value, write a struct literal: `{} {{ ... }}`", name));
            self.err(d);
            return TE::error();
        }
        let mut d = Diagnostic::error(format!("cannot find value `{}` in this scope", name))
            .code("E0302")
            .primary(span, "not found in this scope");
        if BUILTINS.contains(&name) {
            d = d.note(format!("`{}` is a type, not a value", name));
        } else {
            let mut cands: Vec<String> = Vec::new();
            for s in &self.scopes {
                cands.extend(s.keys().cloned());
            }
            cands.extend(self.globals.keys().cloned());
            if let Some(s) = similar(name, cands) {
                d = d.suggest("a local variable with a similar name exists", span, s);
            } else if let Some(h) = type_hint(name) {
                d = d.note(h);
            }
        }
        self.err(d);
        TE::error()
    }

    fn infer_struct_init(&mut self, name: &str, name_span: Span, fields: &[FieldInit], span: Span) -> TE {
        let info_fields: Option<Vec<FieldInfo>> = self.structs.get(name).map(|s| s.fields.clone());
        let info_fields = match info_fields {
            Some(f) => f,
            None => {
                let mut d = Diagnostic::error(format!("cannot find struct `{}` in this scope", name))
                    .code("E0306")
                    .primary(name_span, "not found in this scope");
                if let Some(s) = similar(name, self.structs.keys().cloned().collect()) {
                    d = d.suggest("a struct with a similar name exists", name_span, s);
                }
                self.err(d);
                for f in fields {
                    self.infer(&f.value);
                }
                return TE::error();
            }
        };
        let mut seen: Vec<(String, Span)> = Vec::new();
        for f in fields {
            let te = self.infer(&f.value);
            if let Some((_, prev)) = seen.iter().find(|(n, _)| *n == f.name) {
                let d = Diagnostic::error(format!("field `{}` is specified more than once", f.name))
                    .code("E0308")
                    .primary(f.name_span, "used more than once")
                    .secondary(*prev, "first use of `".to_string() + &f.name + "`");
                self.err(d);
                continue;
            }
            seen.push((f.name.clone(), f.name_span));
            match info_fields.iter().find(|x| x.name == f.name) {
                Some(fi) => {
                    self.coerce(&te, &fi.ty, f.value.span, Ctx::note(fi.ty_span, "expected due to the type of this field"));
                }
                None => {
                    let mut d = Diagnostic::error(format!("struct `{}` has no field named `{}`", name, f.name))
                        .code("E0307")
                        .primary(f.name_span, "unknown field");
                    d = self.field_hint(d, &f.name, f.name_span, &info_fields);
                    self.err(d);
                }
            }
        }
        let _ = span;
        TE::new(RType::Struct(name.to_string()))
    }

    fn field_hint(&self, d: Diagnostic, name: &str, span: Span, fields: &[FieldInfo]) -> Diagnostic {
        if fields.is_empty() {
            return d.note("this struct has no fields");
        }
        let names: Vec<String> = fields.iter().map(|f| f.name.clone()).collect();
        match similar(name, names.clone()) {
            Some(s) => d.suggest("a field with a similar name exists", span, s),
            None => d.note(format!("available fields are: {}", names.join(", "))),
        }
    }

    fn infer_cast(&mut self, ty: &Type, expr: &Expression, span: Span) -> TE {
        let target = self.resolve_type(ty);
        let te = self.infer(expr);
        if target.is_error() || te.ty.is_error() {
            return TE::new(target);
        }
        if te.ty == RType::Void {
            self.void_value_error(expr.span);
            return TE::new(target);
        }
        if !cast_ok(&te.ty, &target) {
            let mut d = Diagnostic::error(format!("invalid cast from {} to `{}`", found_desc(&te), type_name(&target)))
                .code("E0407")
                .primary(span, "this cast is not allowed");
            if target.is_aggregate() || te.ty.is_aggregate() {
                d = d.note("only numbers, booleans and pointers can be cast");
            } else {
                d = d.note("floats and pointers cannot be converted into each other; go through an integer first");
            }
            self.err(d);
        }
        TE::new(target)
    }

    fn infer_unary(&mut self, op: &str, expr: &Expression, span: Span) -> TE {
        match op {
            "-" => {
                let te = self.infer(expr);
                if te.ty.is_error() {
                    return te;
                }
                if let Lit::Int(v) = te.lit {
                    return TE::lit(RType::I64, Lit::Int(-v));
                }
                if te.lit == Lit::Float {
                    return te;
                }
                let ok = te.ty.is_signed_int() || te.ty.is_float();
                if !ok {
                    let mut d = Diagnostic::error(format!("cannot apply unary operator `-` to type `{}`", type_name(&te.ty)))
                        .code("E0414")
                        .primary(span, "cannot be negated");
                    if te.ty.is_int() {
                        d = d.note("unsigned values cannot be negated");
                    }
                    self.err(d);
                    return TE::error();
                }
                TE::new(te.ty)
            }
            "~" => {
                let te = self.infer(expr);
                if te.ty.is_error() {
                    return te;
                }
                if let Lit::Int(v) = te.lit {
                    return TE::lit(RType::I64, Lit::Int(!v));
                }
                if !te.ty.is_int() || te.lit != Lit::None {
                    let d = Diagnostic::error(format!("cannot apply unary operator `~` to {}", found_desc(&te)))
                        .code("E0414")
                        .primary(span, "bitwise not only works on integers");
                    self.err(d);
                    return TE::error();
                }
                TE::new(te.ty)
            }
            "!" => {
                let te = self.infer(expr);
                self.require_scalar(&te, expr.span, "operand of `!`");
                TE::new(RType::Bool)
            }
            "&" => match self.infer_place(expr, "take the address of") {
                Some(t) if !t.is_error() => TE::new(RType::Pointer(Box::new(t))),
                _ => TE::error(),
            },
            "*" => {
                let te = self.infer(expr);
                match &te.ty {
                    RType::Error => TE::error(),
                    RType::Pointer(inner) => TE::new((**inner).clone()),
                    RType::RawPtr if te.lit == Lit::None => {
                        let d = Diagnostic::error("type `rawptr` cannot be dereferenced")
                            .code("E0404")
                            .primary(span, "`rawptr` does not say what it points to")
                            .help("cast it to a typed pointer first, e.g. `*cast(*u8) p`");
                        self.err(d);
                        TE::error()
                    }
                    other => {
                        let d = Diagnostic::error(format!("type {} cannot be dereferenced", found_desc(&te)))
                            .code("E0404")
                            .primary(span, "only pointers can be dereferenced");
                        let _ = other;
                        self.err(d);
                        TE::error()
                    }
                }
            }
            _ => {
                self.infer(expr);
                let d = Diagnostic::error(format!("unknown unary operator `{}`", op)).code("E0414").primary(span, "unknown operator");
                self.err(d);
                TE::error()
            }
        }
    }

    /// Types an expression that must designate a memory location.
    fn infer_place(&mut self, e: &Expression, action: &str) -> Option<RType> {
        if is_place_kind(e) {
            let te = self.infer(e);
            return Some(te.ty);
        }
        self.infer(e);
        let (code, msg, label) = if action == "assign to" {
            ("E0408", "invalid left-hand side of assignment".to_string(), "cannot be assigned to")
        } else {
            ("E0412", format!("cannot {} a temporary value", action), "this value has no address")
        };
        let mut d = Diagnostic::error(msg).code(code).primary(e.span, label);
        if matches!(e.kind, ExprKind::Call { .. }) {
            d = d.note("a call produces a temporary value");
        }
        d = d.note("only variables, `*pointer`, `array[index]` and `value.field` can be used here");
        self.err(d);
        None
    }

    fn infer_assign(&mut self, op: &str, op_span: Span, target: &Expression, value: &Expression, span: Span) -> TE {
        let tty = self.infer_place(target, "assign to");
        let vte = self.infer(value);
        let tty = match tty {
            Some(t) if !t.is_error() => t,
            _ => return TE::error(),
        };
        if op == "=" {
            let ctx = Ctx::note(target.span, format!("this has type `{}`", type_name(&tty)));
            self.coerce(&vte, &tty, value.span, ctx);
            return TE::new(tty);
        }
        if tty.is_aggregate() {
            let d = Diagnostic::error(format!("cannot apply `{}` to `{}`", op, type_name(&tty)))
                .code("E0406")
                .primary(target.span, "compound assignment only works on numbers and pointers");
            self.err(d);
            return TE::error();
        }
        let bin_op = &op[..op.len() - 1];
        let ltv = TE::new(tty.clone());
        let res = self.binary_types(bin_op, op_span, &ltv, target.span, &vte, value.span, span, Some(value));
        if !res.ty.is_error() {
            // `x += y` stores the result back into `x`, which must be able to hold it.
            let ctx = Ctx::note(target.span, format!("this has type `{}`", type_name(&tty)));
            self.coerce(&TE::new(res.ty.clone()), &tty, span, ctx);
        }
        TE::new(tty)
    }

    fn infer_binary(&mut self, op: &str, op_span: Span, left: &Expression, right: &Expression, span: Span) -> TE {
        if op == "&&" || op == "||" {
            let l = self.infer(left);
            let r = self.infer(right);
            self.require_scalar(&l, left.span, "operand");
            self.require_scalar(&r, right.span, "operand");
            return TE::new(RType::Bool);
        }
        let l = self.infer(left);
        let r = self.infer(right);
        self.binary_types(op, op_span, &l, left.span, &r, right.span, span, Some(right))
    }

    #[allow(clippy::too_many_arguments)]
    fn binary_types(&mut self, op: &str, op_span: Span, l: &TE, lspan: Span, r: &TE, rspan: Span, span: Span, rhs_expr: Option<&Expression>) -> TE {
        let _ = (span, rhs_expr);
        if l.ty.is_error() || r.ty.is_error() {
            return TE::error();
        }
        for (t, s) in [(l, lspan), (r, rspan)] {
            if t.ty == RType::Void {
                self.void_value_error(s);
                return TE::error();
            }
            if t.ty.is_aggregate() {
                let d = Diagnostic::error(format!("cannot apply binary operator `{}` to type `{}`", op, type_name(&t.ty)))
                    .code("E0406")
                    .primary(s, format!("`{}` is a struct, array or slice", type_name(&t.ty)))
                    .secondary(op_span, "operator applied here");
                self.err(d);
                return TE::error();
            }
        }
        let is_cmp = matches!(op, "==" | "!=" | "<" | "<=" | ">" | ">=");
        let is_bit = matches!(op, "&" | "|" | "^");
        let is_shift = matches!(op, "<<" | ">>");

        if is_bit || is_shift {
            for (t, s) in [(l, lspan), (r, rspan)] {
                let int_like = t.ty.is_int() || matches!(t.lit, Lit::Int(_));
                if !int_like || t.lit == Lit::Float {
                    let d = Diagnostic::error(format!("cannot apply binary operator `{}` to {}", op, found_desc(t)))
                        .code("E0406")
                        .primary(s, "bitwise operators only work on integers")
                        .secondary(op_span, "operator applied here");
                    self.err(d);
                    return TE::error();
                }
            }
            if let (Lit::Int(a), Lit::Int(b)) = (&l.lit, &r.lit) {
                let folded = match op {
                    "&" => Some(a & b),
                    "|" => Some(a | b),
                    "^" => Some(a ^ b),
                    "<<" if (0..64).contains(b) => a.checked_shl(*b as u32),
                    ">>" if (0..64).contains(b) => Some(a >> *b),
                    _ => None,
                };
                return match folded {
                    Some(v) => TE::lit(RType::I64, Lit::Int(v)),
                    None => TE::new(RType::I64),
                };
            }
            if is_shift {
                // The result has the type of the left operand.
                if let Lit::Int(_) = l.lit {
                    return TE::new(RType::I64);
                }
                return TE::new(l.ty.clone());
            }
        }

        // Function pointers only support `==` and `!=`, against another function
        // pointer of the same type or `null`.
        if l.ty.is_func() || r.ty.is_func() {
            let other_ok = |a: &TE, b: &TE| a.ty == b.ty || matches!(b.lit, Lit::Null | Lit::Int(0));
            if matches!(op, "==" | "!=") && (other_ok(l, r) || other_ok(r, l)) {
                return TE::new(RType::Bool);
            }
            let d = Diagnostic::error(format!("cannot apply binary operator `{}` to `{}` and {}", op, type_name(&l.ty), found_desc(r)))
                .code("E0406")
                .primary(op_span, "invalid operation on a function pointer")
                .note("function pointers can only be compared with `==`/`!=` against another function pointer of the same type or `null`");
            self.err(d);
            return TE::error();
        }

        // Pointer operands.
        if l.ty.is_pointer() || r.ty.is_pointer() {
            return self.pointer_binary(op, is_cmp, op_span, l, lspan, r, rspan);
        }

        // Folding of two integer literals keeps `var x: u8 = 2 * 8` working.
        if let (Lit::Int(a), Lit::Int(b)) = (&l.lit, &r.lit) {
            if is_cmp {
                return TE::new(RType::Bool);
            }
            if (op == "/" || op == "%") && *b == 0 {
                self.div_zero(op, rspan, op_span);
                return TE::error();
            }
            let folded = match op {
                "+" => a.checked_add(*b),
                "-" => a.checked_sub(*b),
                "*" => a.checked_mul(*b),
                "/" => a.checked_div(*b),
                "%" => a.checked_rem(*b),
                _ => None,
            };
            return match folded {
                Some(v) => TE::lit(RType::I64, Lit::Int(v)),
                None => TE::new(RType::I64),
            };
        }

        // Booleans only take part in comparisons.
        if (l.ty == RType::Bool || r.ty == RType::Bool) && !is_cmp {
            let bad = if l.ty == RType::Bool { lspan } else { rspan };
            let d = Diagnostic::error(format!("cannot apply binary operator `{}` to `bool`", op))
                .code("E0406")
                .primary(bad, "booleans do not support arithmetic")
                .secondary(op_span, "operator applied here")
                .help("use `&&` and `||` to combine booleans");
            self.err(d);
            return TE::error();
        }

        // Pick the common type of both operands.
        let common: RType = match (&l.lit, &r.lit) {
            (Lit::Int(_) | Lit::Float, Lit::None) if r.ty.is_int() || r.ty.is_float() || r.ty == RType::Bool => {
                if r.ty == RType::Bool {
                    if !self.coerce(l, &RType::Bool, lspan, Ctx::operand()) {
                        return TE::error();
                    }
                    RType::Bool
                } else {
                    if !self.coerce(l, &r.ty, lspan, Ctx::operand()) {
                        return TE::error();
                    }
                    r.ty.clone()
                }
            }
            (Lit::None, Lit::Int(_) | Lit::Float) if l.ty.is_int() || l.ty.is_float() || l.ty == RType::Bool => {
                if l.ty == RType::Bool {
                    if !self.coerce(r, &RType::Bool, rspan, Ctx::operand()) {
                        return TE::error();
                    }
                    RType::Bool
                } else {
                    if !self.coerce(r, &l.ty, rspan, Ctx::operand()) {
                        return TE::error();
                    }
                    l.ty.clone()
                }
            }
            (Lit::Float, Lit::Int(_)) | (Lit::Int(_), Lit::Float) => RType::F64,
            (Lit::Float, Lit::Float) => RType::F64,
            _ => {
                if l.ty == r.ty {
                    l.ty.clone()
                } else if implicit_ok(&l.ty, &r.ty) {
                    r.ty.clone()
                } else if implicit_ok(&r.ty, &l.ty) {
                    l.ty.clone()
                } else {
                    self.binop_mismatch(op, op_span, l, lspan, r, rspan);
                    return TE::error();
                }
            }
        };

        if op == "%" && common.is_float() {
            let d = Diagnostic::error("cannot apply `%` to floating point numbers")
                .code("E0406")
                .primary(op_span, "the remainder operator only works on integers");
            self.err(d);
            return TE::error();
        }
        if (op == "/" || op == "%") && common.is_int() {
            if let Lit::Int(0) = r.lit {
                self.div_zero(op, rspan, op_span);
                return TE::error();
            }
        }
        if is_cmp {
            TE::new(RType::Bool)
        } else {
            TE::new(common)
        }
    }

    fn div_zero(&mut self, op: &str, rspan: Span, op_span: Span) {
        let what = if op == "/" { "divide" } else { "take the remainder" };
        let d = Diagnostic::error(format!("this operation will {} by zero", what))
            .code("E0406")
            .primary(rspan, "the divisor is zero")
            .secondary(op_span, "operation applied here");
        self.err(d);
    }

    fn binop_mismatch(&mut self, op: &str, op_span: Span, l: &TE, lspan: Span, r: &TE, rspan: Span) {
        let d = Diagnostic::error("mismatched types")
            .code("E0402")
            .primary(rspan, format!("expected `{}`, found {}", type_name(&l.ty), found_desc(r)))
            .secondary(lspan, format!("this has type `{}`", type_name(&l.ty)))
            .note(format!("both operands of `{}` must have the same type, or one must widen losslessly into the other", op))
            .help("convert one side explicitly with `cast(T) value`");
        let _ = op_span;
        self.err(d);
    }

    #[allow(clippy::too_many_arguments)]
    fn pointer_binary(&mut self, op: &str, is_cmp: bool, op_span: Span, l: &TE, lspan: Span, r: &TE, rspan: Span) -> TE {
        let lp = l.ty.is_pointer();
        let rp = r.ty.is_pointer();
        let is_int = |t: &TE| matches!(t.lit, Lit::Int(_)) || (t.ty.is_int() && t.lit == Lit::None);

        if is_cmp {
            if lp && rp {
                if implicit_ok(&l.ty, &r.ty) || implicit_ok(&r.ty, &l.ty) {
                    return TE::new(RType::Bool);
                }
                self.binop_mismatch(op, op_span, l, lspan, r, rspan);
                return TE::error();
            }
            let (ptr, other, ospan) = if lp { (l, r, rspan) } else { (r, l, lspan) };
            if matches!(other.lit, Lit::Int(0)) {
                return TE::new(RType::Bool);
            }
            let d = Diagnostic::error("mismatched types")
                .code("E0402")
                .primary(ospan, format!("expected `{}`, found {}", type_name(&ptr.ty), found_desc(other)))
                .secondary(if lp { lspan } else { rspan }, format!("this has type `{}`", type_name(&ptr.ty)))
                .note("a pointer can only be compared with another pointer or with `0`/`null`");
            self.err(d);
            return TE::error();
        }
        match op {
            "+" if lp && is_int(r) => return TE::new(l.ty.clone()),
            "+" if rp && is_int(l) => return TE::new(r.ty.clone()),
            "-" if lp && is_int(r) => return TE::new(l.ty.clone()),
            "-" if lp && rp => {
                if implicit_ok(&l.ty, &r.ty) || implicit_ok(&r.ty, &l.ty) {
                    return TE::new(RType::I64);
                }
                self.binop_mismatch(op, op_span, l, lspan, r, rspan);
                return TE::error();
            }
            _ => {}
        }
        let bad = if lp { l } else { r };
        let d = Diagnostic::error(format!("cannot apply binary operator `{}` to `{}` and {}", op, type_name(&l.ty), found_desc(r)))
            .code("E0406")
            .primary(op_span, "invalid operation on a pointer")
            .secondary(if lp { lspan } else { rspan }, format!("this has type `{}`", type_name(&bad.ty)))
            .note("pointers support `+ integer`, `- integer`, `pointer - pointer` and comparisons");
        self.err(d);
        TE::error()
    }

    /// Calls through a value of function-pointer type (`f(x)` where `f` is a
    /// variable, `h.cb(x)` for a field, `table[i](x)`, ...).
    fn infer_indirect_call(&mut self, callee: &Expression, args: &[Expression]) -> TE {
        let ct = self.infer(callee);
        let (params, ret) = match &ct.ty {
            RType::Func(p, r) => (p.clone(), (**r).clone()),
            RType::Error => {
                for a in args {
                    self.infer(a);
                }
                return TE::error();
            }
            _ => {
                for a in args {
                    self.infer(a);
                }
                let mut d = Diagnostic::error(format!("expected function, found {}", found_desc(&ct)))
                    .code("E0304")
                    .primary(callee.span, format!("this has type `{}`, which cannot be called", type_name(&ct.ty)));
                if let ExprKind::Member { base, member, .. } = &callee.kind {
                    d = d.help(format!("Marrow has no methods: call a function with the value as an argument, e.g. `{}({}, ...)`", member, self.text(base.span)));
                }
                self.err(d);
                return TE::error();
            }
        };
        let tes: Vec<TE> = args.iter().map(|a| self.infer(a)).collect();
        if args.len() != params.len() {
            let plural = |n: usize| if n == 1 { format!("{} argument", n) } else { format!("{} arguments", n) };
            let was = if args.len() == 1 { "was" } else { "were" };
            let d = Diagnostic::error(format!("this function takes {} but {} {} supplied", plural(params.len()), plural(args.len()), was))
                .code("E0403")
                .primary(callee.span, format!("called through a value of type `{}`", type_name(&ct.ty)));
            self.err(d);
            return TE::new(ret);
        }
        for (i, p) in params.iter().enumerate() {
            self.coerce(&tes[i], p, args[i].span, Ctx::none());
        }
        TE::new(ret)
    }

    fn infer_call(&mut self, callee: &Expression, args: &[Expression], _span: Span) -> TE {
        let via_value = match &callee.kind {
            ExprKind::Identifier(n) => self.lookup_var(n).is_some(),
            _ => true,
        };
        if via_value {
            return self.infer_indirect_call(callee, args);
        }
        let (name, name_span) = match &callee.kind {
            ExprKind::Identifier(n) => (n.clone(), callee.span),
            _ => unreachable!(),
        };
        let sig = match self.funcs.get(&name).cloned() {
            Some(s) => s,
            None => {
                for a in args {
                    self.infer(a);
                }
                if let Some(v) = self.lookup_var(&name) {
                    let d = Diagnostic::error(format!("expected function, found variable `{}`", name))
                        .code("E0304")
                        .primary(name_span, format!("`{}` is a variable of type `{}`", name, type_name(&v.ty)))
                        .secondary(v.name_span, "variable declared here")
                        .note("function pointers are not supported yet");
                    self.err(d);
                } else {
                    let mut d = Diagnostic::error(format!("cannot find function `{}` in this scope", name))
                        .code("E0303")
                        .primary(name_span, "not found in this scope");
                    if let Some(s) = similar(&name, self.funcs.keys().cloned().collect()) {
                        d = d.suggest("a function with a similar name exists", name_span, s);
                    } else {
                        d = d.help("is the file that defines it imported with `@import(\"...\")`?");
                    }
                    self.err(d);
                }
                return TE::error();
            }
        };

        let tes: Vec<TE> = args.iter().map(|a| self.infer(a)).collect();
        let n_params = sig.params.len();
        let bad_count = if sig.variadic { args.len() < n_params } else { args.len() != n_params };
        if bad_count {
            let plural = |n: usize| if n == 1 { format!("{} argument", n) } else { format!("{} arguments", n) };
            let takes = if sig.variadic { format!("at least {}", plural(n_params)) } else { plural(n_params) };
            let was = if args.len() == 1 { "was" } else { "were" };
            let mut d = Diagnostic::error(format!("this function takes {} but {} {} supplied", takes, plural(args.len()), was)).code("E0403");
            if args.len() < n_params {
                let missing = n_params - args.len();
                let label = if missing == 1 {
                    format!("an argument of type `{}` is missing", type_name(&sig.params[args.len()].ty))
                } else {
                    format!("{} arguments are missing", missing)
                };
                d = d.primary(name_span, label);
            } else {
                d = d.primary(name_span, "");
                for (i, a) in args.iter().enumerate().skip(n_params) {
                    d = d.secondary(a.span, format!("unexpected argument #{}", i + 1));
                }
            }
            d = d.secondary(sig.name_span, format!("function `{}` defined here", name));
            self.err(d);
            return TE::new(sig.ret);
        }
        for (i, p) in sig.params.iter().enumerate() {
            let ctx = Ctx::note(p.span, "expected due to this parameter");
            self.coerce(&tes[i], &p.ty, args[i].span, ctx);
        }
        for (te, a) in tes.iter().zip(args.iter()).skip(n_params) {
            if te.ty.is_error() {
                continue;
            }
            if te.ty == RType::Void {
                self.void_value_error(a.span);
            } else if te.ty.is_aggregate() {
                let d = Diagnostic::error("aggregate values cannot be passed as variadic arguments")
                    .code("E0415")
                    .primary(a.span, format!("`{}` is a struct, array or slice", type_name(&te.ty)))
                    .help("pass a pointer to it instead");
                self.err(d);
            } else if let Lit::Int(v) = te.lit {
                if v < i64::MIN as i128 || v > i64::MAX as i128 {
                    self.range_error(v, &RType::I64, a.span, true);
                }
            }
        }
        TE::new(sig.ret)
    }

    fn infer_index(&mut self, base: &Expression, index: &Expression, span: Span) -> TE {
        let b = self.infer(base);
        let i = self.infer(index);
        if !i.ty.is_error() {
            let int_like = matches!(i.lit, Lit::Int(_)) || (i.ty.is_int() && i.lit == Lit::None);
            if !int_like {
                let d = Diagnostic::error("the index must be an integer")
                    .code("E0405")
                    .primary(index.span, format!("found {}", found_desc(&i)));
                self.err(d);
            }
        }
        match &b.ty {
            RType::Error => TE::error(),
            RType::Array(n, elem) => {
                if let Lit::Int(v) = i.lit {
                    if v < 0 || v >= *n as i128 {
                        let d = Diagnostic::error("index out of bounds")
                            .code("E0405")
                            .primary(index.span, format!("the length is {} but the index is {}", n, v))
                            .secondary(base.span, format!("this has type `{}`", type_name(&b.ty)));
                        self.err(d);
                    }
                }
                TE::new((**elem).clone())
            }
            RType::Slice(elem) | RType::Pointer(elem) => TE::new((**elem).clone()),
            RType::RawPtr => {
                let d = Diagnostic::error("cannot index into a `rawptr`")
                    .code("E0405")
                    .primary(span, "`rawptr` does not say what it points to")
                    .help("cast it to a typed pointer first, e.g. `cast(*u8) p`");
                self.err(d);
                TE::error()
            }
            _ => {
                let d = Diagnostic::error(format!("cannot index into a value of type `{}`", type_name(&b.ty)))
                    .code("E0405")
                    .primary(base.span, "this type cannot be indexed")
                    .note("only arrays, slices and typed pointers can be indexed");
                self.err(d);
                TE::error()
            }
        }
    }

    fn infer_member(&mut self, base: &Expression, member: &str, member_span: Span, span: Span) -> TE {
        let b = self.infer(base);
        let _ = span;
        let (sname, via_ptr) = match &b.ty {
            RType::Error => return TE::error(),
            RType::Struct(n) => (n.clone(), false),
            RType::Pointer(inner) => match inner.as_ref() {
                RType::Struct(n) => (n.clone(), true),
                _ => return self.no_field(member, member_span, base.span, &b.ty),
            },
            RType::Slice(elem) => {
                return match member {
                    "ptr" => TE::new(RType::Pointer(elem.clone())),
                    "len" => TE::new(RType::I64),
                    _ => {
                        let d = Diagnostic::error(format!("no field `{}` on type `{}`", member, type_name(&b.ty)))
                            .code("E0307")
                            .primary(member_span, "unknown field")
                            .note("slices have two fields: `ptr` and `len`");
                        self.err(d);
                        TE::error()
                    }
                };
            }
            _ => return self.no_field(member, member_span, base.span, &b.ty),
        };
        let fields: Vec<FieldInfo> = self.structs.get(&sname).map(|s| s.fields.clone()).unwrap_or_default();
        match fields.iter().find(|f| f.name == member) {
            Some(f) => TE::new(f.ty.clone()),
            None => {
                let shown = if via_ptr { format!("*{}", sname) } else { sname.clone() };
                let mut d = Diagnostic::error(format!("no field `{}` on type `{}`", member, shown))
                    .code("E0307")
                    .primary(member_span, "unknown field");
                d = self.field_hint(d, member, member_span, &fields);
                self.err(d);
                TE::error()
            }
        }
    }

    fn no_field(&mut self, member: &str, member_span: Span, base_span: Span, ty: &RType) -> TE {
        let mut d = Diagnostic::error(format!("no field `{}` on type `{}`", member, type_name(ty)))
            .code("E0307")
            .primary(member_span, "unknown field")
            .secondary(base_span, format!("this has type `{}`", type_name(ty)));
        match ty {
            RType::Array(n, _) => {
                d = d.note(format!("arrays have no fields; the length of this array is {}", n));
                d = d.help("take a slice with `array[..]` to get a `len` field");
            }
            RType::Pointer(inner) if matches!(**inner, RType::Pointer(_)) => {
                d = d.help("dereference the pointer first with `*`");
            }
            RType::RawPtr => {
                d = d.help("cast it to a typed pointer to a struct first");
            }
            _ => {
                d = d.note("only structs, pointers to structs and slices have fields");
            }
        }
        self.err(d);
        TE::error()
    }

    fn infer_slice(&mut self, base: &Expression, start: Option<&Expression>, end: Option<&Expression>, span: Span) -> TE {
        let b = self.infer(base);
        for bound in [start, end].into_iter().flatten() {
            let t = self.infer(bound);
            if !t.ty.is_error() {
                let int_like = matches!(t.lit, Lit::Int(_)) || (t.ty.is_int() && t.lit == Lit::None);
                if !int_like {
                    let d = Diagnostic::error("slice bounds must be integers")
                        .code("E0405")
                        .primary(bound.span, format!("found {}", found_desc(&t)));
                    self.err(d);
                }
            }
        }
        match &b.ty {
            RType::Error => TE::error(),
            RType::Array(_, elem) | RType::Slice(elem) => TE::new(RType::Slice(elem.clone())),
            RType::Pointer(elem) => {
                if end.is_none() {
                    let d = Diagnostic::error("missing end bound when slicing a pointer")
                        .code("E0405")
                        .primary(span, "a pointer has no known length")
                        .help("write the end bound explicitly: `ptr[start..end]`");
                    self.err(d);
                }
                TE::new(RType::Slice(elem.clone()))
            }
            _ => {
                let d = Diagnostic::error(format!("cannot slice a value of type `{}`", type_name(&b.ty)))
                    .code("E0405")
                    .primary(base.span, "this type cannot be sliced")
                    .note("only arrays, slices and typed pointers can be sliced");
                self.err(d);
                TE::error()
            }
        }
    }
}

/// `(a + b)` -> true, `(a) + (b)` -> false.
fn fully_parenthesized(text: &str) -> bool {
    let chars: Vec<char> = text.trim().chars().collect();
    if chars.first() != Some(&'(') || chars.last() != Some(&')') {
        return false;
    }
    let mut depth = 0i32;
    for (i, c) in chars.iter().enumerate() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 && i != chars.len() - 1 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

fn dup_error(name: &str, span: Span, prev: Span) -> Diagnostic {
    Diagnostic::error(format!("the name `{}` is defined multiple times", name))
        .code("E0301")
        .primary(span, format!("`{}` redefined here", name))
        .secondary(prev, format!("previous definition of `{}` here", name))
        .note("each name can be defined only once in the program, imports included")
}

fn collect_by_value(t: &RType, out: &mut Vec<String>) {
    match t {
        RType::Struct(n) => out.push(n.clone()),
        RType::Array(_, inner) => collect_by_value(inner, out),
        _ => {}
    }
}

fn type_text_of(text: &str) -> String {
    text.trim().to_string()
}
