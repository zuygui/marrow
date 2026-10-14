//! `marrow --dump-ast <file>`: prints the parsed AST as S-expressions, one top-level
//! item per line. The self-hosted parser (`compiler/astdump.mw`) prints the same
//! format, which is how the two parsers are compared (`tests/selfhost.rs`).
//!
//! Format: `(Kind@start-end "text" child ...)`. Offsets are in characters. A node
//! only carries `@start-end` when the Rust AST has a span for it; absent optional
//! parts are `(None)`.

use crate::ast::*;
use crate::diagnostic::Span;

struct N {
    kind: &'static str,
    span: Option<Span>,
    text: Option<String>,
    kids: Vec<N>,
}

fn n(kind: &'static str, span: Option<Span>, text: Option<String>, kids: Vec<N>) -> N {
    N { kind, span, text, kids }
}

fn none() -> N {
    n("None", None, None, vec![])
}

fn esc(s: &str) -> String {
    let mut o = String::new();
    for &b in s.as_bytes() {
        if (32..127).contains(&b) && b != b'\\' && b != b'"' {
            o.push(b as char);
        } else {
            o.push_str(&format!("\\x{:02x}", b));
        }
    }
    o
}

impl N {
    fn render(&self, out: &mut String) {
        out.push('(');
        out.push_str(self.kind);
        if let Some(s) = self.span {
            out.push_str(&format!("@{}-{}", s.start, s.end));
        }
        if let Some(t) = &self.text {
            out.push_str(&format!(" \"{}\"", esc(t)));
        }
        for k in &self.kids {
            out.push(' ');
            k.render(out);
        }
        out.push(')');
    }
}

pub fn dump_program(p: &Program) -> String {
    let mut out = String::new();
    for item in &p.items {
        item_node(item).render(&mut out);
        out.push('\n');
    }
    out
}

fn item_node(item: &GlobalItem) -> N {
    let mut kids: Vec<N> = item
        .decorators
        .iter()
        .map(|d| n("Decorator", Some(d.span), Some(d.name.clone()), d.args.iter().flatten().map(expr).collect()))
        .collect();
    kids.push(binding(&item.binding));
    n("Item", Some(item.span), None, kids)
}

fn generics(g: &[String]) -> N {
    n("Generics", None, Some(g.join(",")), vec![])
}

fn params(ps: &[Parameter], variadic: bool) -> N {
    let kids = ps.iter().map(|p| n("Param", Some(p.span()), Some(p.name.clone()), vec![ty(&p.ty)])).collect();
    n("Params", None, Some(if variadic { "...".into() } else { String::new() }), kids)
}

fn opt_ty(t: &Option<Type>) -> N {
    t.as_ref().map_or_else(none, ty)
}

fn binding(b: &BindingDecl) -> N {
    let sp = Some(b.span);
    match &b.value {
        BindingValue::Function(f) => {
            let body = match (&f.body, f.body_span) {
                (Some(stmts), span) => n("FnBlock", span, None, stmts.iter().map(stmt).collect()),
                _ => none(),
            };
            n("Fn", sp, Some(b.name.clone()), vec![generics(&b.generics), params(&f.params, f.variadic), opt_ty(&f.ret_type), body])
        }
        BindingValue::ExpressionFunction(f) => n(
            "Fn",
            sp,
            Some(b.name.clone()),
            vec![
                generics(&b.generics),
                params(&f.params, f.variadic),
                opt_ty(&f.ret_type),
                n("ExprBody", None, None, vec![expr(&f.body)]),
            ],
        ),
        BindingValue::Struct(s) => {
            let mut kids = vec![generics(&b.generics)];
            kids.extend(s.fields.iter().map(|f| n("Field", None, Some(f.name.clone()), vec![ty(&f.ty)])));
            n("Struct", sp, Some(b.name.clone()), kids)
        }
        BindingValue::Enum(e) => {
            let mut kids = vec![generics(&b.generics)];
            kids.extend(e.variants.iter().map(|v| n("Variant", None, Some(v.name.clone()), v.fields.iter().map(ty).collect())));
            n("Enum", sp, Some(b.name.clone()), kids)
        }
        BindingValue::Expr(e) => n("GlobalVar", sp, Some(b.name.clone()), vec![opt_ty(&b.ty), expr(e)]),
    }
}

fn types(ts: &[Type]) -> N {
    n("Types", None, None, ts.iter().map(ty).collect())
}

fn ty(t: &Type) -> N {
    let sp = Some(t.span);
    match &t.kind {
        TypeKind::Builtin(s) => n("TyBuiltin", sp, Some(s.clone()), vec![]),
        TypeKind::Custom(s) => n("TyCustom", sp, Some(s.clone()), vec![]),
        TypeKind::StaticArray(size, inner) => n("TyArray", sp, None, vec![expr(size), ty(inner)]),
        TypeKind::Pointer(i) => n("TyPtr", sp, None, vec![ty(i)]),
        TypeKind::Slice(i) => n("TySlice", sp, None, vec![ty(i)]),
        TypeKind::Func(ps, r) => n("TyFunc", sp, None, vec![types(ps), r.as_ref().map_or_else(none, |r| ty(r))]),
        TypeKind::Generic(name, args) => n("TyGeneric", sp, Some(name.clone()), args.iter().map(ty).collect()),
    }
}

fn block(b: &Block) -> N {
    n("Block", None, None, b.iter().map(stmt).collect())
}

fn else_branch(e: &Option<ElseBranch>) -> N {
    match e {
        None => none(),
        Some(ElseBranch::Block(b)) => block(b),
        Some(ElseBranch::If(i)) => if_node("ElseIf", None, i),
    }
}

fn if_node(kind: &'static str, span: Option<Span>, i: &IfStmt) -> N {
    n(kind, span, None, vec![expr(&i.cond), block(&i.then_block), else_branch(&i.else_branch)])
}

fn local(kind: &'static str, span: Option<Span>, d: &LocalVarDecl) -> N {
    match d {
        LocalVarDecl::Mutable { ty: t, name, value, .. } => n(kind, span, Some(name.clone()), vec![opt_ty(t), expr(value)]),
        LocalVarDecl::Constant { name, .. } => n("LocalConst", span, Some(name.clone()), vec![]),
    }
}

fn stmt(s: &Statement) -> N {
    let sp = Some(s.span);
    match &s.kind {
        StmtKind::Block(b) => n("BlockStmt", sp, None, b.iter().map(stmt).collect()),
        StmtKind::LocalVarDecl(d) => local("LocalVar", sp, d),
        StmtKind::If(i) => if_node("If", sp, i),
        StmtKind::While(w) => n("While", sp, None, vec![expr(&w.cond), block(&w.body)]),
        StmtKind::For(f) => n(
            "For",
            sp,
            None,
            vec![
                f.init.as_ref().map_or_else(none, |d| local("InitVar", None, d)),
                f.cond.as_ref().map_or_else(none, expr),
                f.post.as_ref().map_or_else(none, expr),
                block(&f.body),
            ],
        ),
        StmtKind::Return(e) => n("Return", sp, None, vec![e.as_ref().map_or_else(none, expr)]),
        StmtKind::Break => n("Break", sp, None, vec![]),
        StmtKind::Continue => n("Continue", sp, None, vec![]),
        StmtKind::Match(m) => {
            let mut kids = vec![expr(&m.scrutinee)];
            for a in &m.arms {
                let pat = match &a.pattern {
                    None => n("Else", None, None, vec![]),
                    Some(p) => n(
                        "Pat",
                        Some(p.span),
                        Some(format!("{}::{}", p.enum_name, p.variant)),
                        vec![types(&p.type_args), n("Binds", None, Some(p.bindings.iter().map(|b| b.0.clone()).collect::<Vec<_>>().join(",")), vec![])],
                    ),
                };
                kids.push(n("Arm", Some(a.span), None, vec![pat, block(&a.body)]));
            }
            n("Match", sp, None, kids)
        }
        StmtKind::Expr(e) => n("ExprStmt", sp, None, vec![expr(e)]),
    }
}

fn opt_expr(e: &Option<Box<Expression>>) -> N {
    e.as_ref().map_or_else(none, |e| expr(e))
}

fn expr(e: &Expression) -> N {
    let sp = Some(e.span);
    match &e.kind {
        ExprKind::Identifier(s) => n("Ident", sp, Some(s.clone()), vec![]),
        ExprKind::IntLiteral(v) => n("Int", sp, Some(v.to_string()), vec![]),
        ExprKind::FloatLiteral(_) => n("Float", sp, None, vec![]),
        ExprKind::StringLiteral(s) => n("Str", sp, Some(s.clone()), vec![]),
        ExprKind::CharLiteral(c) => n("Char", sp, Some((*c as u32).to_string()), vec![]),
        ExprKind::BoolLiteral(b) => n("Bool", sp, Some(b.to_string()), vec![]),
        ExprKind::Null => n("Null", sp, None, vec![]),
        ExprKind::Zeroed => n("Zeroed", sp, None, vec![]),
        ExprKind::StructInit { name, type_args, fields, .. } => {
            let mut kids = vec![types(type_args)];
            kids.extend(fields.iter().map(|f| n("FieldInit", None, Some(f.name.clone()), vec![expr(&f.value)])));
            n("StructInit", sp, Some(name.clone()), kids)
        }
        ExprKind::Cast { ty: t, expr: x } => n("Cast", sp, None, vec![ty(t), expr(x)]),
        ExprKind::Unary { op, expr: x } => n("Unary", sp, Some(op.clone()), vec![expr(x)]),
        ExprKind::Binary { op, left, right, .. } => n("Binary", sp, Some(op.clone()), vec![expr(left), expr(right)]),
        ExprKind::Assign { op, target, value, .. } => n("Assign", sp, Some(op.clone()), vec![expr(target), expr(value)]),
        ExprKind::Call { callee, args } => {
            let mut kids = vec![expr(callee)];
            kids.extend(args.iter().map(expr));
            n("Call", sp, None, kids)
        }
        ExprKind::Index { base, index } => n("Index", sp, None, vec![expr(base), expr(index)]),
        ExprKind::Member { base, member, .. } => n("Member", sp, Some(member.clone()), vec![expr(base)]),
        ExprKind::Slice { base, start, end } => n("Slice", sp, None, vec![expr(base), opt_expr(start), opt_expr(end)]),
        ExprKind::VaStart => n("VaStart", sp, None, vec![]),
        ExprKind::VaArg { list, ty: t } => n("VaArg", sp, None, vec![expr(list), ty(t)]),
        ExprKind::VaEnd(l) => n("VaEnd", sp, None, vec![expr(l)]),
        ExprKind::SizeOf(t) => n("SizeOf", sp, None, vec![ty(t)]),
        ExprKind::GenericRef { name, args } => n("GenericRef", sp, Some(name.clone()), args.iter().map(ty).collect()),
        ExprKind::Path { enum_name, type_args, variant } => {
            n("Path", sp, Some(format!("{}::{}", enum_name, variant)), type_args.iter().map(ty).collect())
        }
    }
}
