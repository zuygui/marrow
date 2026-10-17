//! Lowering pass, run between import resolution and type checking.
//!
//! It removes the three constructs that exist only at the source level:
//!
//! * **Generics.** `fn`, `struct` and `enum` declarations with type parameters
//!   are templates. Every use with explicit type arguments (`Vec<i64>`,
//!   `max::<i64>(a, b)`, `Option::<i64>::Some(3)`) is instantiated once into an
//!   ordinary declaration with a mangled name (`Vec__i64`).
//! * **Enums.** A C-like enum (no payloads) is an `i64`; its variants are integer
//!   constants. An enum with payloads becomes
//!   `struct E { tag: i64; data: [W]u64; }`, one payload struct per variant and
//!   one constructor function `E__Variant` per variant.
//! * **`match`.** Becomes a copy of the scrutinee plus an `if / else if` chain on
//!   the tag; payload fields are copied into the arm's bindings.
//!
//! Everything the later stages see is plain Marrow.

use crate::ast::*;
use crate::diagnostic::{Diagnostic, Span};
use std::collections::{HashMap, HashSet, VecDeque};

type Env = HashMap<String, Type>;

const MAX_INSTANCES: usize = 4000;

#[derive(Clone)]
struct VariantInfo {
    name: String,
    tag: i64,
    fields: Vec<Type>,
    payload: String,
    ctor: String,
}

#[derive(Clone)]
struct EnumInfo {
    name: String,
    c_like: bool,
    variants: Vec<VariantInfo>,
}

#[derive(Clone, Copy, PartialEq)]
enum Want {
    Type,
    Func,
}

struct Inst {
    mangled: String,
    c_like: bool,
}

struct Lowerer {
    templates: HashMap<String, GlobalItem>,
    /// Concrete enums (declared or instantiated): `true` when C-like.
    enum_c_like: HashMap<String, bool>,
    enums: HashMap<String, EnumInfo>,
    inst: HashMap<String, Inst>,
    used: HashSet<String>,
    queue: VecDeque<(String, GlobalItem, Env)>,
    out: Vec<GlobalItem>,
    diags: Vec<Diagnostic>,
    gensym: usize,
    n_inst: usize,
    /// Payload enums whose `data` array length is still to be computed:
    /// (index in `out`, enum name).
    pending_w: Vec<(usize, String)>,
}

pub fn lower(program: Program) -> (Program, Vec<Diagnostic>) {
    let mut lw = Lowerer {
        templates: HashMap::new(),
        enum_c_like: HashMap::new(),
        enums: HashMap::new(),
        inst: HashMap::new(),
        used: HashSet::new(),
        queue: VecDeque::new(),
        out: Vec::new(),
        diags: Vec::new(),
        gensym: 0,
        n_inst: 0,
        pending_w: Vec::new(),
    };
    lw.run(program);
    (Program { items: lw.out }, lw.diags)
}

fn ty(kind: TypeKind, span: Span) -> Type {
    Type { kind, span }
}

fn ex(kind: ExprKind, span: Span) -> Expression {
    Expression { kind, span }
}

fn ident(name: &str, span: Span) -> Expression {
    ex(ExprKind::Identifier(name.to_string()), span)
}

fn member(base: Expression, name: &str, span: Span) -> Expression {
    ex(ExprKind::Member { base: Box::new(base), member: name.to_string(), member_span: span }, span)
}

fn binary(op: &str, l: Expression, r: Expression, span: Span) -> Expression {
    ex(ExprKind::Binary { op: op.to_string(), op_span: span, left: Box::new(l), right: Box::new(r) }, span)
}

fn assign(target: Expression, value: Expression, span: Span) -> Statement {
    let e = ex(ExprKind::Assign { op: "=".to_string(), op_span: span, target: Box::new(target), value: Box::new(value) }, span);
    Statement { kind: StmtKind::Expr(e), span }
}

fn local(name: &str, t: Option<Type>, value: Expression, span: Span) -> Statement {
    Statement {
        kind: StmtKind::LocalVarDecl(LocalVarDecl::Mutable { ty: t, name: name.to_string(), name_span: span, value }),
        span,
    }
}

/// `cast(*Payload) cast(rawptr) &base.data[0]`
fn payload_ptr(base: &str, payload: &str, span: Span) -> Expression {
    let data = member(ident(base, span), "data", span);
    let first = ex(ExprKind::Index { base: Box::new(data), index: Box::new(ex(ExprKind::IntLiteral(0), span)) }, span);
    let addr = ex(ExprKind::Unary { op: "&".to_string(), expr: Box::new(first) }, span);
    let raw = ex(ExprKind::Cast { ty: ty(TypeKind::Builtin("rawptr".into()), span), expr: Box::new(addr) }, span);
    ex(
        ExprKind::Cast {
            ty: ty(TypeKind::Pointer(Box::new(ty(TypeKind::Custom(payload.to_string()), span))), span),
            expr: Box::new(raw),
        },
        span,
    )
}

fn const_size(e: &Expression) -> Option<u64> {
    match &e.kind {
        ExprKind::IntLiteral(v) if *v >= 0 => Some(*v as u64),
        ExprKind::Binary { op, left, right, .. } => {
            let (l, r) = (const_size(left)?, const_size(right)?);
            match op.as_str() {
                "+" => l.checked_add(r),
                "-" => l.checked_sub(r),
                "*" => l.checked_mul(r),
                "/" => l.checked_div(r),
                _ => None,
            }
        }
        _ => None,
    }
}

/// An unambiguous textual key for a (lowered) type.
fn ty_key(t: &Type) -> String {
    match &t.kind {
        TypeKind::Builtin(n) | TypeKind::Custom(n) => n.clone(),
        TypeKind::Pointer(i) => format!("*{}", ty_key(i)),
        TypeKind::Slice(i) => format!("({})[]", ty_key(i)),
        TypeKind::StaticArray(n, i) => format!("[{}]{}", const_size(n).map_or("?".to_string(), |v| v.to_string()), ty_key(i)),
        TypeKind::Func(ps, r) => {
            let ps: Vec<String> = ps.iter().map(ty_key).collect();
            format!("fn({})->{}", ps.join(","), r.as_ref().map_or("void".to_string(), |r| ty_key(r)))
        }
        TypeKind::Generic(n, a) => format!("{}<{}>", n, a.iter().map(ty_key).collect::<Vec<_>>().join(",")),
    }
}

/// A name fragment (letters, digits, `_`) for a lowered type.
fn mangle_ty(t: &Type) -> String {
    match &t.kind {
        TypeKind::Builtin(n) | TypeKind::Custom(n) => n.clone(),
        TypeKind::Pointer(i) => format!("p_{}", mangle_ty(i)),
        TypeKind::Slice(i) => format!("s_{}", mangle_ty(i)),
        TypeKind::StaticArray(n, i) => format!("a{}_{}", const_size(n).unwrap_or(0), mangle_ty(i)),
        TypeKind::Func(ps, r) => {
            let ps: Vec<String> = ps.iter().map(mangle_ty).collect();
            format!("fn_{}_to_{}", ps.join("_"), r.as_ref().map_or("void".to_string(), |r| mangle_ty(r)))
        }
        TypeKind::Generic(n, _) => n.clone(),
    }
}

impl Lowerer {
    fn err(&mut self, d: Diagnostic) {
        self.diags.push(d);
    }

    fn run(&mut self, program: Program) {
        let mut kept = Vec::new();
        for item in program.items {
            if item.binding.generics.is_empty() {
                kept.push(item);
                continue;
            }
            let name = item.binding.name.clone();
            if let Some(prev) = self.templates.get(&name) {
                let d = Diagnostic::error(format!("the generic `{}` is defined more than once", name))
                    .code("E0423")
                    .primary(item.binding.name_span, "redefined here")
                    .secondary(prev.binding.name_span, "first defined here");
                self.err(d);
                continue;
            }
            self.templates.insert(name, item);
        }

        for item in &kept {
            if let BindingValue::Enum(ed) = &item.binding.value {
                self.enum_c_like.insert(item.binding.name.clone(), ed.variants.iter().all(|v| v.fields.is_empty()));
            }
        }

        // Enums first: their infos are needed to lower `match` anywhere else.
        let env = Env::new();
        let mut rest = Vec::new();
        for mut item in kept {
            if let BindingValue::Enum(ed) = &mut item.binding.value {
                for v in &mut ed.variants {
                    for f in &mut v.fields {
                        self.rewrite_type(f, &env);
                    }
                }
                self.register_enum(&item);
            } else {
                rest.push(item);
            }
        }
        for mut item in rest {
            self.rewrite_item(&mut item, &env);
            self.out.push(item);
        }

        while let Some((mangled, mut item, env)) = self.queue.pop_front() {
            item.binding.name = mangled;
            item.binding.generics.clear();
            self.rewrite_item(&mut item, &env);
            self.out.push(item);
        }

        self.finish_enums();
    }

    // -- instantiation -----------------------------------------------------

    fn instantiate(&mut self, name: &str, args: &[Type], span: Span, want: Want) -> Option<Inst> {
        let Some(tpl) = self.templates.get(name).cloned() else {
            let d = Diagnostic::error(format!("`{}` is not a generic {}", name, if want == Want::Type { "type" } else { "function" }))
                .code("E0423")
                .primary(span, "type arguments given here");
            self.err(d);
            return None;
        };
        let is_fn = matches!(tpl.binding.value, BindingValue::Function(_) | BindingValue::ExpressionFunction(_));
        if is_fn != (want == Want::Func) {
            let d = Diagnostic::error(format!("`{}` is a generic {}, not a {}", name, if is_fn { "function" } else { "type" }, if is_fn { "type" } else { "function" }))
                .code("E0423")
                .primary(span, "used as the wrong kind of item")
                .secondary(tpl.binding.name_span, "declared here");
            self.err(d);
            return None;
        }
        let params = tpl.binding.generics.clone();
        if params.len() != args.len() {
            let d = Diagnostic::error(format!("`{}` takes {} type argument(s) but {} were given", name, params.len(), args.len()))
                .code("E0422")
                .primary(span, format!("expected {}", params.len()))
                .secondary(tpl.binding.name_span, format!("`{}` declared here with <{}>", name, params.join(", ")));
            self.err(d);
            return None;
        }
        let key = format!("{}<{}>", name, args.iter().map(ty_key).collect::<Vec<_>>().join(","));
        if let Some(i) = self.inst.get(&key) {
            return Some(Inst { mangled: i.mangled.clone(), c_like: i.c_like });
        }
        self.n_inst += 1;
        if self.n_inst > MAX_INSTANCES {
            if self.n_inst == MAX_INSTANCES + 1 {
                let d = Diagnostic::error("too many generic instantiations")
                    .code("E0430")
                    .primary(span, "instantiation limit reached")
                    .note("this usually means a generic keeps instantiating itself with ever larger types");
                self.err(d);
            }
            return None;
        }
        let base = format!("{}__{}", name, args.iter().map(mangle_ty).collect::<Vec<_>>().join("__"));
        let mut mangled = base.clone();
        let mut n = 1;
        while self.used.contains(&mangled) {
            n += 1;
            mangled = format!("{}_{}", base, n);
        }
        self.used.insert(mangled.clone());

        let c_like = match &tpl.binding.value {
            BindingValue::Enum(ed) => ed.variants.iter().all(|v| v.fields.is_empty()),
            _ => false,
        };
        self.inst.insert(key, Inst { mangled: mangled.clone(), c_like });
        let env: Env = params.into_iter().zip(args.iter().cloned()).collect();

        if let BindingValue::Enum(_) = &tpl.binding.value {
            self.enum_c_like.insert(mangled.clone(), c_like);
            let mut item = tpl;
            item.binding.name = mangled.clone();
            item.binding.generics.clear();
            if let BindingValue::Enum(ed) = &mut item.binding.value {
                for v in &mut ed.variants {
                    for f in &mut v.fields {
                        self.rewrite_type(f, &env);
                    }
                }
            }
            self.register_enum(&item);
        } else {
            self.queue.push_back((mangled.clone(), tpl, env));
        }
        Some(Inst { mangled, c_like })
    }

    // -- enums -------------------------------------------------------------

    fn register_enum(&mut self, item: &GlobalItem) {
        let BindingValue::Enum(ed) = &item.binding.value else { return };
        let name = item.binding.name.clone();
        let span = item.binding.name_span;
        let c_like = ed.variants.iter().all(|v| v.fields.is_empty());
        let variants: Vec<VariantInfo> = ed
            .variants
            .iter()
            .enumerate()
            .map(|(i, v)| VariantInfo {
                name: v.name.clone(),
                tag: i as i64,
                fields: v.fields.clone(),
                payload: format!("{}__{}_P", name, v.name),
                ctor: format!("{}__{}", name, v.name),
            })
            .collect();
        let mut seen = HashSet::new();
        for v in &ed.variants {
            if !seen.insert(v.name.clone()) {
                let d = Diagnostic::error(format!("variant `{}` is declared twice in `{}`", v.name, name)).code("E0425").primary(v.name_span, "duplicate variant");
                self.err(d);
            }
        }
        self.enum_c_like.insert(name.clone(), c_like);
        let info = EnumInfo { name: name.clone(), c_like, variants };
        if !c_like {
            self.emit_enum_items(&info, item, span);
        }
        self.enums.insert(name, info);
    }

    fn emit_enum_items(&mut self, info: &EnumInfo, item: &GlobalItem, span: Span) {
        let sp = item.span;
        let mk = |binding: BindingDecl| GlobalItem { decorators: Vec::new(), binding, span: sp };
        let enum_ty = || ty(TypeKind::Custom(info.name.clone()), span);

        // The enum itself; `data`'s length is patched in `finish_enums`.
        let data_ty = ty(
            TypeKind::StaticArray(Box::new(ex(ExprKind::IntLiteral(1), span)), Box::new(ty(TypeKind::Builtin("u64".into()), span))),
            span,
        );
        let fields = vec![
            StructField { ty: ty(TypeKind::Builtin("i64".into()), span), name: "tag".into(), name_span: span },
            StructField { ty: data_ty, name: "data".into(), name_span: span },
        ];
        self.pending_w.push((self.out.len(), info.name.clone()));
        self.out.push(mk(BindingDecl {
            generics: Vec::new(),
            ty: None,
            name: info.name.clone(),
            name_span: span,
            value: BindingValue::Struct(StructDef { fields }),
            span: sp,
        }));

        for v in &info.variants {
            if !v.fields.is_empty() {
                let fields = v
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(i, t)| StructField { ty: t.clone(), name: format!("f{}", i), name_span: span })
                    .collect();
                self.out.push(mk(BindingDecl {
                    generics: Vec::new(),
                    ty: None,
                    name: v.payload.clone(),
                    name_span: span,
                    value: BindingValue::Struct(StructDef { fields }),
                    span: sp,
                }));
            }
            // fn Enum__Variant(a0: T0, ...) -> Enum { var r: Enum; r.tag = k; ...; ret r; }
            let params: Vec<Parameter> = v
                .fields
                .iter()
                .enumerate()
                .map(|(i, t)| Parameter { ty: t.clone(), name: format!("a{}", i), name_span: span })
                .collect();
            let mut body: Block = vec![
                local("r", Some(enum_ty()), ex(ExprKind::Zeroed, span), span),
                assign(member(ident("r", span), "tag", span), ex(ExprKind::IntLiteral(v.tag as i128), span), span),
            ];
            if !v.fields.is_empty() {
                body.push(local("p", Some(ty(TypeKind::Pointer(Box::new(ty(TypeKind::Custom(v.payload.clone()), span))), span)), payload_ptr("r", &v.payload, span), span));
                for i in 0..v.fields.len() {
                    body.push(assign(member(ident("p", span), &format!("f{}", i), span), ident(&format!("a{}", i), span), span));
                }
            }
            body.push(Statement { kind: StmtKind::Return(Some(ident("r", span))), span });
            self.out.push(mk(BindingDecl {
                generics: Vec::new(),
                ty: None,
                name: v.ctor.clone(),
                name_span: span,
                value: BindingValue::Function(FunctionDef {
                    params,
                    ret_type: Some(enum_ty()),
                    variadic: false,
                    body: Some(body),
                    body_span: Some(span),
                }),
                span: sp,
            }));
        }
    }

    /// Computes the length of every payload enum's `data` array: the size of its
    /// largest payload struct, in 8-byte words.
    fn finish_enums(&mut self) {
        let mut defs: HashMap<String, Vec<Type>> = HashMap::new();
        for it in &self.out {
            if let BindingValue::Struct(sd) = &it.binding.value {
                defs.insert(it.binding.name.clone(), sd.fields.iter().map(|f| f.ty.clone()).collect());
            }
        }
        let pending: HashMap<String, usize> = self.pending_w.iter().map(|(i, n)| (n.clone(), *i)).collect();
        let mut words: HashMap<String, u64> = HashMap::new();
        let names: Vec<String> = self.pending_w.iter().map(|(_, n)| n.clone()).collect();
        for n in names {
            let mut visiting = Vec::new();
            if let Err(msg) = self.enum_words(&n, &pending, &defs, &mut words, &mut visiting) {
                let span = self.out[pending[&n]].binding.name_span;
                let d = Diagnostic::error(msg).code("E0430").primary(span, "in this enum").help("store recursive values behind a pointer: `*T`");
                self.err(d);
            }
        }
        for (n, w) in &words {
            let idx = pending[n];
            if let BindingValue::Struct(sd) = &mut self.out[idx].binding.value {
                let span = sd.fields[1].name_span;
                sd.fields[1].ty = ty(
                    TypeKind::StaticArray(Box::new(ex(ExprKind::IntLiteral(*w as i128), span)), Box::new(ty(TypeKind::Builtin("u64".into()), span))),
                    span,
                );
            }
        }
    }

    fn enum_words(
        &self,
        name: &str,
        pending: &HashMap<String, usize>,
        defs: &HashMap<String, Vec<Type>>,
        words: &mut HashMap<String, u64>,
        visiting: &mut Vec<String>,
    ) -> Result<u64, String> {
        if let Some(w) = words.get(name) {
            return Ok(*w);
        }
        if visiting.iter().any(|v| v == name) {
            return Err(format!("enum `{}` contains itself and would have infinite size", name));
        }
        visiting.push(name.to_string());
        let mut max = 0u64;
        if let Some(info) = self.enums.get(name) {
            for v in &info.variants {
                if v.fields.is_empty() {
                    continue;
                }
                let (sz, _) = self.size_align(&ty(TypeKind::Custom(v.payload.clone()), Span::DUMMY), pending, defs, words, visiting)?;
                max = max.max(sz);
            }
        }
        visiting.pop();
        let w = max.div_ceil(8).max(1);
        words.insert(name.to_string(), w);
        Ok(w)
    }

    /// Size and alignment of a lowered type, mirroring the code generator's layout.
    fn size_align(
        &self,
        t: &Type,
        pending: &HashMap<String, usize>,
        defs: &HashMap<String, Vec<Type>>,
        words: &mut HashMap<String, u64>,
        visiting: &mut Vec<String>,
    ) -> Result<(u64, u64), String> {
        Ok(match &t.kind {
            TypeKind::Builtin(n) => match n.as_str() {
                "i8" | "u8" | "bool" => (1, 1),
                "i16" | "u16" => (2, 2),
                "i32" | "u32" | "f32" => (4, 4),
                _ => (8, 8),
            },
            TypeKind::Pointer(_) | TypeKind::Func(_, _) => (8, 8),
            TypeKind::Slice(_) => (16, 8),
            TypeKind::StaticArray(n, inner) => {
                let (s, a) = self.size_align(inner, pending, defs, words, visiting)?;
                (s.saturating_mul(const_size(n).unwrap_or(0)), a)
            }
            TypeKind::Generic(_, _) => (8, 8),
            TypeKind::Custom(n) => {
                if pending.contains_key(n) {
                    let w = self.enum_words(n, pending, defs, words, visiting)?;
                    (8 + w * 8, 8)
                } else if let Some(fields) = defs.get(n) {
                    if visiting.iter().any(|v| v == n) {
                        return Err(format!("struct `{}` contains itself and would have infinite size", n));
                    }
                    visiting.push(n.clone());
                    let (mut off, mut al) = (0u64, 1u64);
                    for f in fields {
                        let (s, a) = self.size_align(f, pending, defs, words, visiting)?;
                        off = off.div_ceil(a) * a + s;
                        al = al.max(a);
                    }
                    visiting.pop();
                    (off.div_ceil(al) * al, al)
                } else {
                    (8, 8)
                }
            }
        })
    }

    // -- types -------------------------------------------------------------

    fn rewrite_type(&mut self, t: &mut Type, env: &Env) {
        let span = t.span;
        match &mut t.kind {
            TypeKind::Builtin(_) => {}
            TypeKind::Custom(n) => {
                if let Some(r) = env.get(n.as_str()) {
                    t.kind = r.kind.clone();
                } else if self.enum_c_like.get(n.as_str()) == Some(&true) {
                    t.kind = TypeKind::Builtin("i64".into());
                } else if let Some(tpl) = self.templates.get(n.as_str()) {
                    let params = tpl.binding.generics.join(", ");
                    let d = Diagnostic::error(format!("generic type `{}` needs type arguments", n))
                        .code("E0422")
                        .primary(span, format!("write `{}<{}>` with concrete types", n, params));
                    self.err(d);
                }
            }
            TypeKind::Generic(n, args) => {
                for a in args.iter_mut() {
                    self.rewrite_type(a, env);
                }
                let (n, args) = (n.clone(), args.clone());
                t.kind = match self.instantiate(&n, &args, span, Want::Type) {
                    Some(i) if i.c_like => TypeKind::Builtin("i64".into()),
                    Some(i) => TypeKind::Custom(i.mangled),
                    None => TypeKind::Builtin("i64".into()),
                };
            }
            TypeKind::StaticArray(size, inner) => {
                self.rewrite_expr(size, env);
                self.rewrite_type(inner, env);
            }
            TypeKind::Pointer(i) | TypeKind::Slice(i) => self.rewrite_type(i, env),
            TypeKind::Func(ps, r) => {
                for p in ps.iter_mut() {
                    self.rewrite_type(p, env);
                }
                if let Some(r) = r {
                    self.rewrite_type(r, env);
                }
            }
        }
    }

    // -- items -------------------------------------------------------------

    fn rewrite_item(&mut self, item: &mut GlobalItem, env: &Env) {
        if let Some(t) = &mut item.binding.ty {
            self.rewrite_type(t, env);
        }
        self.rewrite_value(&mut item.binding.value, env);
    }

    fn rewrite_value(&mut self, v: &mut BindingValue, env: &Env) {
        match v {
            BindingValue::Function(fd) => {
                for p in &mut fd.params {
                    self.rewrite_type(&mut p.ty, env);
                }
                if let Some(r) = &mut fd.ret_type {
                    self.rewrite_type(r, env);
                }
                if let Some(b) = &mut fd.body {
                    self.rewrite_block(b, env);
                }
            }
            BindingValue::ExpressionFunction(fd) => {
                for p in &mut fd.params {
                    self.rewrite_type(&mut p.ty, env);
                }
                if let Some(r) = &mut fd.ret_type {
                    self.rewrite_type(r, env);
                }
                self.rewrite_expr(&mut fd.body, env);
            }
            BindingValue::Struct(sd) => {
                for f in &mut sd.fields {
                    self.rewrite_type(&mut f.ty, env);
                }
            }
            BindingValue::Enum(ed) => {
                for v in &mut ed.variants {
                    for f in &mut v.fields {
                        self.rewrite_type(f, env);
                    }
                }
            }
            BindingValue::Expr(e) => self.rewrite_expr(e, env),
        }
    }

    // -- statements --------------------------------------------------------

    fn rewrite_block(&mut self, b: &mut Block, env: &Env) {
        for s in b.iter_mut() {
            self.rewrite_stmt(s, env);
        }
    }

    fn rewrite_decl(&mut self, d: &mut LocalVarDecl, env: &Env) {
        match d {
            LocalVarDecl::Mutable { ty, value, .. } => {
                if let Some(t) = ty {
                    self.rewrite_type(t, env);
                }
                self.rewrite_expr(value, env);
            }
            LocalVarDecl::Constant { ty, value, .. } => {
                if let Some(t) = ty {
                    self.rewrite_type(t, env);
                }
                self.rewrite_value(value, env);
            }
        }
    }

    fn rewrite_if(&mut self, i: &mut IfStmt, env: &Env) {
        self.rewrite_expr(&mut i.cond, env);
        self.rewrite_block(&mut i.then_block, env);
        match &mut i.else_branch {
            None => {}
            Some(ElseBranch::Block(b)) => self.rewrite_block(b, env),
            Some(ElseBranch::If(inner)) => self.rewrite_if(inner, env),
        }
    }

    fn rewrite_stmt(&mut self, s: &mut Statement, env: &Env) {
        match &mut s.kind {
            StmtKind::Block(b) => self.rewrite_block(b, env),
            StmtKind::LocalVarDecl(d) => self.rewrite_decl(d, env),
            StmtKind::If(i) => self.rewrite_if(i, env),
            StmtKind::While(w) => {
                self.rewrite_expr(&mut w.cond, env);
                self.rewrite_block(&mut w.body, env);
            }
            StmtKind::For(f) => {
                if let Some(init) = &mut f.init {
                    self.rewrite_decl(init, env);
                }
                if let Some(c) = &mut f.cond {
                    self.rewrite_expr(c, env);
                }
                if let Some(p) = &mut f.post {
                    self.rewrite_expr(p, env);
                }
                self.rewrite_block(&mut f.body, env);
            }
            StmtKind::Return(e) => {
                if let Some(e) = e {
                    self.rewrite_expr(e, env);
                }
            }
            StmtKind::Break | StmtKind::Continue => {}
            StmtKind::Expr(e) => self.rewrite_expr(e, env),
            StmtKind::Match(_) => {
                let span = s.span;
                let StmtKind::Match(m) = std::mem::replace(&mut s.kind, StmtKind::Break) else { unreachable!() };
                s.kind = self.lower_match(m, span, env);
            }
        }
    }

    // -- expressions -------------------------------------------------------

    fn rewrite_exprs(&mut self, v: &mut [Expression], env: &Env) {
        for e in v {
            self.rewrite_expr(e, env);
        }
    }

    fn rewrite_expr(&mut self, e: &mut Expression, env: &Env) {
        let span = e.span;
        match &mut e.kind {
            ExprKind::Identifier(_)
            | ExprKind::IntLiteral(_)
            | ExprKind::FloatLiteral(_)
            | ExprKind::StringLiteral(_)
            | ExprKind::CharLiteral(_)
            | ExprKind::BoolLiteral(_)
            | ExprKind::Null
            | ExprKind::Zeroed
            | ExprKind::VaStart => {}
            ExprKind::SizeOf(t) => self.rewrite_type(t, env),
            ExprKind::Cast { ty, expr } => {
                self.rewrite_type(ty, env);
                self.rewrite_expr(expr, env);
            }
            ExprKind::Unary { expr, .. } | ExprKind::VaEnd(expr) => self.rewrite_expr(expr, env),
            ExprKind::VaArg { list, ty } => {
                self.rewrite_expr(list, env);
                self.rewrite_type(ty, env);
            }
            ExprKind::Binary { left, right, .. } => {
                self.rewrite_expr(left, env);
                self.rewrite_expr(right, env);
            }
            ExprKind::Assign { target, value, .. } => {
                self.rewrite_expr(target, env);
                self.rewrite_expr(value, env);
            }
            ExprKind::Index { base, index } => {
                self.rewrite_expr(base, env);
                self.rewrite_expr(index, env);
            }
            ExprKind::Member { base, .. } => self.rewrite_expr(base, env),
            ExprKind::Slice { base, start, end } => {
                self.rewrite_expr(base, env);
                if let Some(s) = start {
                    self.rewrite_expr(s, env);
                }
                if let Some(en) = end {
                    self.rewrite_expr(en, env);
                }
            }
            ExprKind::Call { callee, args } => {
                self.rewrite_exprs(args, env);
                match &callee.kind {
                    ExprKind::Path { .. } => self.rewrite_path(callee, env, true),
                    ExprKind::Identifier(n) if self.templates.contains_key(n.as_str()) && !env.contains_key(n.as_str()) => {
                        let tpl = &self.templates[n.as_str()];
                        let d = Diagnostic::error(format!("generic function `{}` needs explicit type arguments", n))
                            .code("E0422")
                            .primary(callee.span, format!("write `{}::<{}>(...)`", n, tpl.binding.generics.join(", ")));
                        self.err(d);
                    }
                    _ => self.rewrite_expr(callee, env),
                }
            }
            ExprKind::StructInit { name, type_args, fields, name_span } => {
                for f in fields.iter_mut() {
                    self.rewrite_expr(&mut f.value, env);
                }
                if !type_args.is_empty() {
                    for a in type_args.iter_mut() {
                        self.rewrite_type(a, env);
                    }
                    let (n, args, nsp) = (name.clone(), type_args.clone(), *name_span);
                    match self.instantiate(&n, &args, nsp, Want::Type) {
                        Some(i) => {
                            *name = i.mangled;
                            type_args.clear();
                        }
                        None => type_args.clear(),
                    }
                } else if let Some(r) = env.get(name.as_str()) {
                    if let TypeKind::Custom(m) = &r.kind {
                        *name = m.clone();
                    }
                } else if let Some(tpl) = self.templates.get(name.as_str()) {
                    let d = Diagnostic::error(format!("generic type `{}` needs type arguments", name))
                        .code("E0422")
                        .primary(*name_span, format!("write `{}::<{}> {{ ... }}`", name, tpl.binding.generics.join(", ")));
                    self.err(d);
                }
            }
            ExprKind::GenericRef { name, args } => {
                for a in args.iter_mut() {
                    self.rewrite_type(a, env);
                }
                let (n, args) = (name.clone(), args.clone());
                e.kind = match self.instantiate(&n, &args, span, Want::Func) {
                    Some(i) => ExprKind::Identifier(i.mangled),
                    None => ExprKind::Null,
                };
            }
            ExprKind::Path { .. } => self.rewrite_path(e, env, false),
        }
    }

    fn resolve_enum(&mut self, name: &str, type_args: &mut Vec<Type>, span: Span, env: &Env) -> Option<EnumInfo> {
        for a in type_args.iter_mut() {
            self.rewrite_type(a, env);
        }
        if type_args.is_empty() {
            if let Some(info) = self.enums.get(name) {
                return Some(info.clone());
            }
            let d = if let Some(tpl) = self.templates.get(name) {
                Diagnostic::error(format!("generic enum `{}` needs type arguments", name))
                    .code("E0422")
                    .primary(span, format!("write `{}::<{}>::Variant`", name, tpl.binding.generics.join(", ")))
            } else {
                Diagnostic::error(format!("unknown enum `{}`", name)).code("E0424").primary(span, "not an enum")
            };
            self.err(d);
            return None;
        }
        let inst = self.instantiate(name, type_args, span, Want::Type)?;
        match self.enums.get(&inst.mangled) {
            Some(i) => Some(i.clone()),
            None => {
                let d = Diagnostic::error(format!("`{}` is not an enum", name)).code("E0424").primary(span, "not an enum");
                self.err(d);
                None
            }
        }
    }

    fn rewrite_path(&mut self, e: &mut Expression, env: &Env, as_callee: bool) {
        let span = e.span;
        let ExprKind::Path { enum_name, type_args, variant } = &mut e.kind else { return };
        let (enum_name, variant) = (enum_name.clone(), variant.clone());
        let mut targs = std::mem::take(type_args);
        let Some(info) = self.resolve_enum(&enum_name, &mut targs, span, env) else {
            e.kind = ExprKind::IntLiteral(0);
            return;
        };
        let Some(v) = info.variants.iter().find(|v| v.name == variant) else {
            let names: Vec<&str> = info.variants.iter().map(|v| v.name.as_str()).collect();
            let d = Diagnostic::error(format!("enum `{}` has no variant `{}`", enum_name, variant))
                .code("E0425")
                .primary(span, "unknown variant")
                .help(format!("available variants: {}", names.join(", ")));
            self.err(d);
            e.kind = ExprKind::IntLiteral(0);
            return;
        };
        e.kind = if info.c_like {
            ExprKind::IntLiteral(v.tag as i128)
        } else if as_callee {
            ExprKind::Identifier(v.ctor.clone())
        } else if v.fields.is_empty() {
            ExprKind::Call { callee: Box::new(ident(&v.ctor, span)), args: Vec::new() }
        } else {
            let d = Diagnostic::error(format!("variant `{}::{}` carries {} value(s)", enum_name, variant, v.fields.len()))
                .code("E0429")
                .primary(span, format!("call it: `{}::{}(...)`", enum_name, variant));
            self.err(d);
            ExprKind::IntLiteral(0)
        };
    }

    // -- match -------------------------------------------------------------

    fn lower_match(&mut self, mut m: MatchStmt, span: Span, env: &Env) -> StmtKind {
        self.rewrite_expr(&mut m.scrutinee, env);

        // Resolve every pattern; they must all name the same enum.
        let mut info: Option<EnumInfo> = None;
        let mut resolved: Vec<Option<usize>> = Vec::new(); // variant index per arm
        for arm in &mut m.arms {
            let Some(p) = &mut arm.pattern else {
                resolved.push(None);
                continue;
            };
            let mut targs = std::mem::take(&mut p.type_args);
            let (pname, pspan) = (p.enum_name.clone(), p.span);
            let Some(i) = self.resolve_enum(&pname, &mut targs, pspan, env) else {
                resolved.push(None);
                continue;
            };
            if let Some(first) = &info {
                if first.name != i.name {
                    let d = Diagnostic::error(format!("this pattern is a `{}` but earlier arms match `{}`", i.name, first.name))
                        .code("E0426")
                        .primary(pspan, "different enum");
                    self.err(d);
                    resolved.push(None);
                    continue;
                }
            } else {
                info = Some(i.clone());
            }
            match i.variants.iter().position(|v| v.name == p.variant) {
                Some(idx) => resolved.push(Some(idx)),
                None => {
                    let names: Vec<&str> = i.variants.iter().map(|v| v.name.as_str()).collect();
                    let d = Diagnostic::error(format!("enum `{}` has no variant `{}`", p.enum_name, p.variant))
                        .code("E0425")
                        .primary(pspan, "unknown variant")
                        .help(format!("available variants: {}", names.join(", ")));
                    self.err(d);
                    resolved.push(None);
                }
            }
        }

        // `else` must be last and unique.
        let n_arms = m.arms.len();
        for (i, arm) in m.arms.iter().enumerate() {
            if arm.pattern.is_none() && i + 1 != n_arms {
                let d = Diagnostic::error("the `else` arm must be the last arm of a `match`").code("E0427").primary(arm.span, "arms after this one are unreachable");
                self.err(d);
            }
        }

        // Lower the arm bodies first.
        for arm in &mut m.arms {
            self.rewrite_block(&mut arm.body, env);
        }

        let Some(info) = info else {
            // Only an `else` arm (or errors already reported).
            return match m.arms.pop() {
                Some(arm) => StmtKind::Block(arm.body),
                None => {
                    let d = Diagnostic::error("`match` without arms").code("E0428").primary(span, "add at least one arm");
                    self.err(d);
                    StmtKind::Block(Vec::new())
                }
            };
        };

        let has_else = m.arms.iter().any(|a| a.pattern.is_none());
        let mut covered = HashSet::new();
        for (arm, r) in m.arms.iter().zip(&resolved) {
            if let Some(idx) = r {
                if !covered.insert(*idx) {
                    let d = Diagnostic::warning(format!("variant `{}` is already matched by an earlier arm", info.variants[*idx].name))
                        .code("W0006")
                        .primary(arm.span, "this arm is never selected");
                    self.err(d);
                }
            }
        }
        if !has_else && covered.len() < info.variants.len() {
            let missing: Vec<String> = info.variants.iter().enumerate().filter(|(i, _)| !covered.contains(i)).map(|(_, v)| format!("`{}::{}`", info.name, v.name)).collect();
            let d = Diagnostic::error(format!("non-exhaustive `match`: {} not covered", missing.join(", ")))
                .code("E0428")
                .primary(span, "some variants are not handled")
                .help("add arms for the missing variants, or an `else` arm");
            self.err(d);
        }

        self.gensym += 1;
        let tmp = format!("__m{}", self.gensym);
        let scrut_ty = if info.c_like { TypeKind::Builtin("i64".into()) } else { TypeKind::Custom(info.name.clone()) };
        let decl = local(&tmp, Some(ty(scrut_ty, span)), m.scrutinee, span);

        // (condition, body) per arm, in order.
        let mut chain: Vec<(Option<Expression>, Block)> = Vec::new();
        for (arm, r) in m.arms.into_iter().zip(resolved) {
            let asp = arm.span;
            let (cond, mut body) = match (&arm.pattern, r) {
                (Some(p), Some(idx)) => {
                    let v = &info.variants[idx];
                    let lhs = if info.c_like { ident(&tmp, asp) } else { member(ident(&tmp, asp), "tag", asp) };
                    let cond = binary("==", lhs, ex(ExprKind::IntLiteral(v.tag as i128), asp), asp);
                    let mut prelude: Block = Vec::new();
                    if p.bindings.len() != v.fields.len() {
                        let d = Diagnostic::error(format!("variant `{}::{}` has {} value(s) but the pattern binds {}", info.name, v.name, v.fields.len(), p.bindings.len()))
                            .code("E0429")
                            .primary(p.span, format!("expected {} binding(s)", v.fields.len()));
                        self.err(d);
                    } else if !v.fields.is_empty() {
                        self.gensym += 1;
                        let ptr = format!("__p{}", self.gensym);
                        let pty = ty(TypeKind::Pointer(Box::new(ty(TypeKind::Custom(v.payload.clone()), asp))), asp);
                        prelude.push(local(&ptr, Some(pty), payload_ptr(&tmp, &v.payload, asp), asp));
                        for (i, (bname, bspan)) in p.bindings.iter().enumerate() {
                            if bname == "_" {
                                continue;
                            }
                            prelude.push(local(bname, Some(v.fields[i].clone()), member(ident(&ptr, *bspan), &format!("f{}", i), *bspan), *bspan));
                        }
                    }
                    (Some(cond), prelude)
                }
                (None, _) => (None, Vec::new()),
                (Some(_), None) => (Some(ex(ExprKind::BoolLiteral(false), asp)), Vec::new()),
            };
            body.extend(arm.body);
            chain.push((cond, body));
        }

        // The last arm needs no test: the match is exhaustive or has an `else`.
        let mut iter = chain.into_iter().rev();
        let Some((_, last_body)) = iter.next() else { return StmtKind::Block(vec![decl]) };
        let mut tail: Option<ElseBranch> = Some(ElseBranch::Block(last_body));
        let mut result: Option<IfStmt> = None;
        for (cond, body) in iter {
            let i = IfStmt { cond: cond.unwrap_or_else(|| ex(ExprKind::BoolLiteral(true), span)), then_block: body, else_branch: tail.take() };
            tail = Some(ElseBranch::If(Box::new(i.clone())));
            result = Some(i);
        }
        let mut stmts = vec![decl];
        match result {
            Some(i) => stmts.push(Statement { kind: StmtKind::If(i), span }),
            None => {
                if let Some(ElseBranch::Block(b)) = tail {
                    stmts.push(Statement { kind: StmtKind::Block(b), span });
                }
            }
        }
        StmtKind::Block(stmts)
    }
}
