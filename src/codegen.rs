use std::collections::HashMap;
use std::fmt::Write as _;

use crate::ast::*;
use crate::error::CompileError;

#[derive(Debug, Clone, PartialEq)]
pub enum RType {
    I8, I16, I32, I64,
    U8, U16, U32, U64,
    F32, F64,
    Bool,
    RawPtr,
    Pointer(Box<RType>),
    Slice(Box<RType>),
    Array(u64, Box<RType>),
    Struct(String),
    Void,
}

impl RType {
    fn is_aggregate(&self) -> bool {
        matches!(self, RType::Struct(_) | RType::Array(_, _) | RType::Slice(_))
    }
    fn is_float(&self) -> bool {
        matches!(self, RType::F32 | RType::F64)
    }
    fn is_integerish(&self) -> bool {
        matches!(
            self,
            RType::I8 | RType::I16 | RType::I32 | RType::I64
                | RType::U8 | RType::U16 | RType::U32 | RType::U64
                | RType::Bool | RType::RawPtr | RType::Pointer(_)
        )
    }
}

pub fn type_name(rt: &RType) -> String {
    match rt {
        RType::I8 => "i8".to_string(),
        RType::I16 => "i16".to_string(),
        RType::I32 => "i32".to_string(),
        RType::I64 => "i64".to_string(),
        RType::U8 => "u8".to_string(),
        RType::U16 => "u16".to_string(),
        RType::U32 => "u32".to_string(),
        RType::U64 => "u64".to_string(),
        RType::F32 => "f32".to_string(),
        RType::F64 => "f64".to_string(),
        RType::Bool => "bool".to_string(),
        RType::RawPtr => "rawptr".to_string(),
        RType::Pointer(t) => format!("{}*", type_name(t)),
        RType::Slice(t) => format!("[]{}", type_name(t)),
        RType::Array(n, t) => format!("[{}]{}", n, type_name(t)),
        RType::Struct(n) => n.clone(),
        RType::Void => "void".to_string(),
    }
}

fn int_bits(rt: &RType) -> u32 {
    match rt {
        RType::I8 | RType::U8 | RType::Bool => 8,
        RType::I16 | RType::U16 => 16,
        RType::I32 | RType::U32 => 32,
        RType::I64 | RType::U64 | RType::RawPtr => 64,
        RType::Pointer(_) => 64,
        _ => 0,
    }
}

fn is_signed_int(rt: &RType) -> bool {
    matches!(rt, RType::I8 | RType::I16 | RType::I32 | RType::I64)
}

fn base_ty(rt: &RType) -> &'static str {
    match rt {
        RType::I8 | RType::I16 | RType::I32 | RType::U8 | RType::U16 | RType::U32 | RType::Bool => "w",
        RType::I64 | RType::U64 | RType::RawPtr | RType::Pointer(_) => "l",
        RType::F32 => "s",
        RType::F64 => "d",
        _ => "l",
    }
}

fn store_instr(rt: &RType) -> &'static str {
    match rt {
        RType::I8 | RType::U8 | RType::Bool => "storeb",
        RType::I16 | RType::U16 => "storeh",
        RType::I32 | RType::U32 => "storew",
        RType::I64 | RType::U64 | RType::RawPtr | RType::Pointer(_) => "storel",
        RType::F32 => "stores",
        RType::F64 => "stored",
        _ => "storel",
    }
}

fn load_instr(rt: &RType) -> &'static str {
    match rt {
        RType::I8 => "loadsb",
        RType::U8 => "loadub",
        RType::Bool => "loadub",
        RType::I16 => "loadsh",
        RType::U16 => "loaduh",
        RType::I32 => "loadsw",
        RType::U32 => "loaduw",
        RType::I64 | RType::U64 | RType::RawPtr | RType::Pointer(_) => "loadl",
        RType::F32 => "loads",
        RType::F64 => "loadd",
        _ => "loadl",
    }
}

fn promote(ty: &RType) -> RType {
    match ty {
        RType::I8 | RType::I16 | RType::Bool => RType::I32,
        RType::U8 | RType::U16 => RType::U32,
        other => other.clone(),
    }
}

fn variadic_promote(ty: &RType) -> RType {
    match ty {
        RType::F32 => RType::F64,
        other => promote(other),
    }
}

fn cmp_instr(op: &str, ty: &RType) -> String {
    let bty = base_ty(ty);
    if ty.is_float() {
        let part = match op {
            "==" => "eq", "!=" => "ne", "<" => "lt", "<=" => "le", ">" => "gt", ">=" => "ge",
            _ => "eq",
        };
        format!("c{}{}", part, bty)
    } else {
        let signed = is_signed_int(ty);
        let part = match op {
            "==" => "eq".to_string(),
            "!=" => "ne".to_string(),
            "<" => if signed { "slt".to_string() } else { "ult".to_string() },
            "<=" => if signed { "sle".to_string() } else { "ule".to_string() },
            ">" => if signed { "sgt".to_string() } else { "ugt".to_string() },
            ">=" => if signed { "sge".to_string() } else { "uge".to_string() },
            _ => "eq".to_string(),
        };
        format!("c{}{}", part, bty)
    }
}

fn alloc_instr_for_align(al: u64) -> &'static str {
    if al <= 4 { "alloc4" } else if al <= 8 { "alloc8" } else { "alloc16" }
}

fn align_up(off: u64, al: u64) -> u64 {
    if al == 0 { off } else { (off + al - 1) / al * al }
}

fn find_decorator<'a>(decorators: &'a [Decorator], name: &str) -> Option<&'a Decorator> {
    decorators.iter().find(|d| d.name == name)
}

fn decorator_string_arg(dec: &Decorator, line: usize, col: usize) -> Result<Option<String>, CompileError> {
    match &dec.args {
        None => Ok(None),
        Some(args) if args.is_empty() => Ok(None),
        Some(args) if args.len() == 1 => match &args[0].kind {
            ExprKind::StringLiteral(s) => Ok(Some(s.clone())),
            _ => Err(CompileError::new(line, col, 1, format!("'@{}' : l'argument doit être une chaîne littérale (le nom du symbole lié)", dec.name))),
        },
        Some(_) => Err(CompileError::new(line, col, 1, format!("'@{}' accepte au plus un argument (le nom du symbole lié)", dec.name))),
    }
}

fn fn_linkage(decorators: &[Decorator], name: &str, line: usize, col: usize) -> Result<(bool, bool, String), CompileError> {
    let extern_dec = find_decorator(decorators, "extern");
    let is_extern = extern_dec.is_some();
    let is_export = find_decorator(decorators, "export").is_some();
    let symbol = match extern_dec {
        Some(dec) => decorator_string_arg(dec, line, col)?.unwrap_or_else(|| name.to_string()),
        None => name.to_string(),
    };
    Ok((is_extern, is_export, format!("${}", symbol)))
}

#[derive(Debug, Clone)]
pub struct StructLayout {
    pub fields: Vec<(String, RType, u64)>,
    pub size: u64,
    pub align: u64,
}


#[derive(Debug, Clone)]
struct FnSig {
    qbe_name: String,
    params: Vec<RType>,
    ret: RType,
    variadic: bool,
}

#[derive(Clone)]
enum ConstBinding {
    Func(String, Vec<RType>, RType, bool),
    TypeAlias(String),
}

#[derive(Default)]
struct Scope {
    vars: HashMap<String, (String, RType)>,
    consts: HashMap<String, ConstBinding>,
}

#[derive(Clone)]
enum CgValue {
    Scalar(String, RType),
    Address(String, RType),
}

impl CgValue {
    fn ty(&self) -> RType {
        match self {
            CgValue::Scalar(_, t) => t.clone(),
            CgValue::Address(_, t) => t.clone(),
        }
    }
}

enum ConstVal {
    Int(i128),
    Float(f64),
    Bool(bool),
    Str(String),
    Null,
}


pub struct Codegen {
    structs: HashMap<String, Option<StructLayout>>,
    funcs: HashMap<String, FnSig>,
    global_consts: HashMap<String, (String, RType)>,
    strings: HashMap<String, String>,
    data: Vec<String>,
    functions: Vec<String>,
    tmp: usize,
    lbl: usize,
    anon: usize,
    scopes: Vec<Scope>,
    cur: String,
    terminated: bool,
    ret_type: RType,
    sret: Option<String>,
}

pub fn generate(program: &Program) -> Result<String, CompileError> {
    let mut cg = Codegen {
        structs: HashMap::new(),
        funcs: HashMap::new(),
        global_consts: HashMap::new(),
        strings: HashMap::new(),
        data: Vec::new(),
        functions: Vec::new(),
        tmp: 0,
        lbl: 0,
        anon: 0,
        scopes: Vec::new(),
        cur: String::new(),
        terminated: false,
        ret_type: RType::Void,
        sret: None,
    };
    cg.run(program)?;

    let mut out = String::new();
    out.push_str("# --- généré par marrow (backend QBE) ---\n\n");
    for d in &cg.data {
        out.push_str(d);
        out.push('\n');
    }
    if !cg.data.is_empty() {
        out.push('\n');
    }
    for f in &cg.functions {
        out.push_str(f);
        out.push('\n');
    }
    Ok(out)
}

impl Codegen {

    fn run(&mut self, program: &Program) -> Result<(), CompileError> {
        for item in &program.items {
            if let BindingValue::Struct(_) = &item.binding.value {
                self.structs.insert(item.binding.name.clone(), None);
            }
        }

        for item in &program.items {
            if let BindingValue::Struct(sd) = &item.binding.value {
                self.resolve_struct_layout(&item.binding.name, &sd.fields)?;
            }
        }

        for item in &program.items {
            match &item.binding.value {
                BindingValue::Function(fd) => {
                    let (is_extern, _is_export, qbe_name) = fn_linkage(&item.decorators, &item.binding.name, item.line, item.col)?;
                    if is_extern && fd.body.is_some() {
                        return Err(CompileError::new(
                            item.line,
                            item.col,
                            item.binding.name.chars().count().max(1),
                            format!("'{}' est décorée '@extern' : elle ne doit pas avoir de corps (retirez le '{{...}}' / le '=> ...')", item.binding.name),
                        ));
                    }
                    if !is_extern && fd.body.is_none() {
                        return Err(CompileError::new(
                            item.line,
                            item.col,
                            item.binding.name.chars().count().max(1),
                            format!("il manque un corps à la fonction '{}' (ajoutez '{{ ... }}', ou décorez-la avec '@extern' si elle est définie ailleurs)", item.binding.name),
                        ));
                    }
                    let params = self.resolve_param_types(&fd.params)?;
                    let ret = self.combined_ret_type(item.binding.ty.as_ref(), fd.ret_type.as_ref())?;
                    self.funcs.insert(item.binding.name.clone(), FnSig { qbe_name, params, ret, variadic: fd.variadic });
                }
                BindingValue::ExpressionFunction(efd) => {
                    if find_decorator(&item.decorators, "extern").is_some() {
                        return Err(CompileError::new(
                            item.line,
                            item.col,
                            1,
                            "'@extern' n'est pas compatible avec une fonction à corps expression ('=>') ; utilisez la forme '(...) -> Type;' sans corps",
                        ));
                    }
                    let (_, _is_export, qbe_name) = fn_linkage(&item.decorators, &item.binding.name, item.line, item.col)?;
                    let params = self.resolve_param_types(&efd.params)?;
                    let ret = self.combined_ret_type(item.binding.ty.as_ref(), efd.ret_type.as_ref())?;
                    self.funcs.insert(item.binding.name.clone(), FnSig { qbe_name, params, ret, variadic: efd.variadic });
                }
                BindingValue::Struct(_) => {
                    if find_decorator(&item.decorators, "extern").is_some() || find_decorator(&item.decorators, "export").is_some() {
                        return Err(CompileError::new(item.line, item.col, 1, "'@extern'/'@export' ne s'appliquent qu'aux fonctions"));
                    }
                }
                BindingValue::Expr(e) => {
                    if find_decorator(&item.decorators, "extern").is_some() {
                        return Err(CompileError::new(item.line, item.col, 1, "'@extern' ne s'applique pas aux constantes globales (seules les fonctions peuvent l'être)"));
                    }
                    let is_export = find_decorator(&item.decorators, "export").is_some();
                    let ty = match &item.binding.ty {
                        Some(t) => self.resolve_type(t)?,
                        None => self.infer_const_type(e)?,
                    };
                    let cv = self.const_fold(e)?;
                    let sym = format!("${}", item.binding.name);
                    self.emit_global_const(&sym, &ty, &cv, is_export, item.line, item.col)?;
                    self.global_consts.insert(item.binding.name.clone(), (sym, ty));
                }
            }
        }

        for item in &program.items {
            match &item.binding.value {
                BindingValue::Function(fd) => {
                    let (is_extern, is_export, qbe_name) = fn_linkage(&item.decorators, &item.binding.name, item.line, item.col)?;
                    if is_extern {
                        continue;
                    }
                    let ret = self.combined_ret_type(item.binding.ty.as_ref(), fd.ret_type.as_ref())?;
                    let body = fd.body.as_ref().expect("validé en passe 3 : non-extern implique un corps présent");
                    self.gen_function(&qbe_name, is_export, &fd.params, fd.variadic, ret, body)?;
                }
                BindingValue::ExpressionFunction(efd) => {
                    let (_, is_export, qbe_name) = fn_linkage(&item.decorators, &item.binding.name, item.line, item.col)?;
                    let ret = self.combined_ret_type(item.binding.ty.as_ref(), efd.ret_type.as_ref())?;
                    let ret_stmt = Statement {
                        kind: StmtKind::Return(Some((*efd.body).clone())),
                        line: item.line,
                        col: item.col,
                    };
                    let body = vec![ret_stmt];
                    self.gen_function(&qbe_name, is_export, &efd.params, efd.variadic, ret, &body)?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn resolve_param_types(&self, params: &[Parameter]) -> Result<Vec<RType>, CompileError> {
        let mut out = Vec::with_capacity(params.len());
        for p in params {
            out.push(self.resolve_type(&p.ty)?);
        }
        Ok(out)
    }

    fn resolve_opt_type(&self, ty: Option<&Type>) -> Result<RType, CompileError> {
        match ty {
            Some(t) => self.resolve_type(t),
            None => Ok(RType::Void),
        }
    }

    fn combined_ret_type(&self, decl_ty: Option<&Type>, fn_ret_type: Option<&Type>) -> Result<RType, CompileError> {
        if let Some(t) = fn_ret_type {
            return self.resolve_type(t);
        }
        if let Some(t) = decl_ty {
            return self.resolve_type(t);
        }
        Ok(RType::Void)
    }

    fn resolve_custom_name(&self, name: &str) -> Option<String> {
        for s in self.scopes.iter().rev() {
            if let Some(ConstBinding::TypeAlias(m)) = s.consts.get(name) {
                return Some(m.clone());
            }
        }
        if self.structs.contains_key(name) {
            return Some(name.to_string());
        }
        None
    }

    fn resolve_type(&self, ty: &Type) -> Result<RType, CompileError> {
        match ty {
            Type::Builtin(s) => builtin_rtype(s)
                .ok_or_else(|| CompileError::new(0, 0, 1, format!("type de base inconnu : '{}'", s))),
            Type::Custom(name) => {
                let resolved = self.resolve_custom_name(name).ok_or_else(|| {
                    CompileError::new(0, 0, name.chars().count().max(1), format!("type inconnu : '{}'", name))
                })?;
                Ok(RType::Struct(resolved))
            }
            Type::StaticArray(size_expr, inner) => {
                let n = self.const_eval_int(size_expr)?;
                let inner_rt = self.resolve_type(inner)?;
                Ok(RType::Array(n, Box::new(inner_rt)))
            }
            Type::Pointer(inner) => Ok(RType::Pointer(Box::new(self.resolve_type(inner)?))),
            Type::Slice(inner) => Ok(RType::Slice(Box::new(self.resolve_type(inner)?))),
        }
    }

    fn const_eval_int(&self, e: &Expression) -> Result<u64, CompileError> {
        match &e.kind {
            ExprKind::IntLiteral(v) => {
                if *v < 0 {
                    return Err(CompileError::new(e.line, e.col, 1, "la taille d'un tableau statique ne peut pas être négative"));
                }
                Ok(*v as u64)
            }
            ExprKind::Unary { op, .. } if op == "-" => {
                Err(CompileError::new(e.line, e.col, 1, "la taille d'un tableau statique ne peut pas être négative"))
            }
            ExprKind::Binary { op, left, right } => {
                let l = self.const_eval_int(left)?;
                let r = self.const_eval_int(right)?;
                match op.as_str() {
                    "+" => Ok(l + r),
                    "-" => l.checked_sub(r).ok_or_else(|| {
                        CompileError::new(e.line, e.col, 1, "taille de tableau statique négative")
                    }),
                    "*" => Ok(l * r),
                    "/" => {
                        if r == 0 {
                            Err(CompileError::new(e.line, e.col, 1, "division par zéro dans une taille de tableau constante"))
                        } else {
                            Ok(l / r)
                        }
                    }
                    _ => Err(CompileError::new(e.line, e.col, 1, "expression non constante dans la taille d'un tableau statique")),
                }
            }
            _ => Err(CompileError::new(e.line, e.col, 1, "la taille d'un tableau statique doit être une expression constante entière")),
        }
    }

    fn size_align(&self, rt: &RType) -> Result<(u64, u64), CompileError> {
        Ok(match rt {
            RType::I8 | RType::U8 | RType::Bool => (1, 1),
            RType::I16 | RType::U16 => (2, 2),
            RType::I32 | RType::U32 => (4, 4),
            RType::I64 | RType::U64 | RType::RawPtr => (8, 8),
            RType::F32 => (4, 4),
            RType::F64 => (8, 8),
            RType::Pointer(_) => (8, 8),
            RType::Slice(_) => (16, 8),
            RType::Array(n, inner) => {
                let (es, ea) = self.size_align(inner)?;
                (es.saturating_mul(*n), ea)
            }
            RType::Struct(name) => match self.structs.get(name) {
                Some(Some(layout)) => (layout.size, layout.align),
                Some(None) => {
                    return Err(CompileError::new(0, 0, 1, format!("type récursif de taille infinie : '{}' (utilisez un pointeur)", name)))
                }
                None => return Err(CompileError::new(0, 0, 1, format!("type de structure inconnu : '{}'", name))),
            },
            RType::Void => (0, 1),
        })
    }

    fn resolve_struct_layout(&mut self, name: &str, fields: &[StructField]) -> Result<(), CompileError> {
        self.structs.insert(name.to_string(), None);
        let mut offset: u64 = 0;
        let mut max_align: u64 = 1;
        let mut out_fields: Vec<(String, RType, u64)> = Vec::new();
        for f in fields {
            let rt = self.resolve_type(&f.ty)?;
            let (sz, al) = self.size_align(&rt)?;
            offset = align_up(offset, al);
            out_fields.push((f.name.clone(), rt, offset));
            offset += sz;
            if al > max_align {
                max_align = al;
            }
        }
        let size = align_up(offset, max_align);
        self.structs.insert(name.to_string(), Some(StructLayout { fields: out_fields, size, align: max_align }));
        Ok(())
    }

    fn infer_const_type(&self, e: &Expression) -> Result<RType, CompileError> {
        match &e.kind {
            ExprKind::IntLiteral(_) => Ok(RType::I64),
            ExprKind::FloatLiteral(_) => Ok(RType::F64),
            ExprKind::BoolLiteral(_) => Ok(RType::Bool),
            ExprKind::CharLiteral(_) => Ok(RType::U8),
            ExprKind::StringLiteral(_) => Ok(RType::Pointer(Box::new(RType::U8))),
            ExprKind::Null => Ok(RType::RawPtr),
            ExprKind::Unary { op, expr } if op == "-" => self.infer_const_type(expr),
            ExprKind::Binary { left, .. } => self.infer_const_type(left),
            _ => Err(CompileError::new(e.line, e.col, 1, "impossible de déduire le type de cette constante globale ; précisez un type explicite")),
        }
    }

    fn const_fold(&self, e: &Expression) -> Result<ConstVal, CompileError> {
        match &e.kind {
            ExprKind::IntLiteral(v) => Ok(ConstVal::Int(*v)),
            ExprKind::FloatLiteral(v) => Ok(ConstVal::Float(*v)),
            ExprKind::BoolLiteral(v) => Ok(ConstVal::Bool(*v)),
            ExprKind::CharLiteral(c) => Ok(ConstVal::Int(*c as i128)),
            ExprKind::StringLiteral(s) => Ok(ConstVal::Str(s.clone())),
            ExprKind::Null => Ok(ConstVal::Null),
            ExprKind::Unary { op, expr } if op == "-" => match self.const_fold(expr)? {
                ConstVal::Int(i) => Ok(ConstVal::Int(-i)),
                ConstVal::Float(f) => Ok(ConstVal::Float(-f)),
                _ => Err(CompileError::new(e.line, e.col, 1, "négation d'une constante non numérique")),
            },
            ExprKind::Binary { op, left, right } => {
                let l = self.const_fold(left)?;
                let r = self.const_fold(right)?;
                match (l, r) {
                    (ConstVal::Int(a), ConstVal::Int(b)) => Ok(ConstVal::Int(match op.as_str() {
                        "+" => a + b,
                        "-" => a - b,
                        "*" => a * b,
                        "/" => {
                            if b == 0 {
                                return Err(CompileError::new(e.line, e.col, 1, "division par zéro dans une constante globale"));
                            }
                            a / b
                        }
                        _ => return Err(CompileError::new(e.line, e.col, 1, "opération non supportée dans une constante globale")),
                    })),
                    (ConstVal::Float(a), ConstVal::Float(b)) => Ok(ConstVal::Float(match op.as_str() {
                        "+" => a + b,
                        "-" => a - b,
                        "*" => a * b,
                        "/" => a / b,
                        _ => return Err(CompileError::new(e.line, e.col, 1, "opération non supportée dans une constante globale")),
                    })),
                    _ => Err(CompileError::new(e.line, e.col, 1, "opérandes de types incompatibles dans une constante globale")),
                }
            }
            _ => Err(CompileError::new(
                e.line,
                e.col,
                1,
                "expression non constante : une liaison globale doit avoir une valeur calculable à la compilation",
            )),
        }
    }

    fn const_val_as_i128(&self, cv: &ConstVal, line: usize, col: usize) -> Result<i128, CompileError> {
        match cv {
            ConstVal::Int(i) => Ok(*i),
            ConstVal::Bool(b) => Ok(if *b { 1 } else { 0 }),
            ConstVal::Float(f) => Ok(*f as i128),
            _ => Err(CompileError::new(line, col, 1, "valeur constante entière attendue")),
        }
    }

    fn const_val_as_f64(&self, cv: &ConstVal, line: usize, col: usize) -> Result<f64, CompileError> {
        match cv {
            ConstVal::Float(f) => Ok(*f),
            ConstVal::Int(i) => Ok(*i as f64),
            _ => Err(CompileError::new(line, col, 1, "valeur constante flottante attendue")),
        }
    }

    fn emit_global_const(&mut self, sym: &str, ty: &RType, cv: &ConstVal, exported: bool, line: usize, col: usize) -> Result<(), CompileError> {
        let lk = if exported { "export " } else { "" };
        match ty {
            RType::Pointer(inner) if matches!(**inner, RType::U8) => match cv {
                ConstVal::Str(s) => {
                    let strsym = self.intern_string(s);
                    self.data.push(format!("{}data {} = {{ l {} }}", lk, sym, strsym));
                    Ok(())
                }
                ConstVal::Null => {
                    self.data.push(format!("{}data {} = {{ l 0 }}", lk, sym));
                    Ok(())
                }
                _ => Err(CompileError::new(line, col, 1, "constante globale : une chaîne ou 'null' est attendue pour un type '*u8'")),
            },
            RType::RawPtr | RType::Pointer(_) => match cv {
                ConstVal::Null => {
                    self.data.push(format!("{}data {} = {{ l 0 }}", lk, sym));
                    Ok(())
                }
                ConstVal::Int(i) => {
                    self.data.push(format!("{}data {} = {{ l {} }}", lk, sym, i));
                    Ok(())
                }
                _ => Err(CompileError::new(line, col, 1, "constante globale de type pointeur invalide")),
            },
            RType::F32 => {
                let f = self.const_val_as_f64(cv, line, col)?;
                self.data.push(format!("{}data {} = {{ s s_{} }}", lk, sym, f));
                Ok(())
            }
            RType::F64 => {
                let f = self.const_val_as_f64(cv, line, col)?;
                self.data.push(format!("{}data {} = {{ d d_{} }}", lk, sym, f));
                Ok(())
            }
            RType::Bool => {
                let i = self.const_val_as_i128(cv, line, col)?;
                self.data.push(format!("{}data {} = {{ b {} }}", lk, sym, if i != 0 { 1 } else { 0 }));
                Ok(())
            }
            RType::I8 | RType::U8 => {
                let i = self.const_val_as_i128(cv, line, col)?;
                self.data.push(format!("{}data {} = {{ b {} }}", lk, sym, i));
                Ok(())
            }
            RType::I16 | RType::U16 => {
                let i = self.const_val_as_i128(cv, line, col)?;
                self.data.push(format!("{}data {} = {{ h {} }}", lk, sym, i));
                Ok(())
            }
            RType::I32 | RType::U32 => {
                let i = self.const_val_as_i128(cv, line, col)?;
                self.data.push(format!("{}data {} = {{ w {} }}", lk, sym, i));
                Ok(())
            }
            RType::I64 | RType::U64 => {
                let i = self.const_val_as_i128(cv, line, col)?;
                self.data.push(format!("{}data {} = {{ l {} }}", lk, sym, i));
                Ok(())
            }
            _ => Err(CompileError::new(line, col, 1, "les constantes globales de type agrégé (structure, tableau, slice) ne sont pas supportées")),
        }
    }

    fn intern_string(&mut self, s: &str) -> String {
        if let Some(sym) = self.strings.get(s) {
            return sym.clone();
        }
        self.anon += 1;
        let sym = format!("$str.{}", self.anon);
        self.strings.insert(s.to_string(), sym.clone());
        let mut items: Vec<String> = s.bytes().map(|b| b.to_string()).collect();
        items.push("0".to_string());
        let def = format!("data {} = {{ b {} }}", sym, items.join(" "));
        self.data.push(def);
        sym
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope::default());
    }
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }
    fn declare_var(&mut self, name: &str, addr: String, ty: RType) {
        self.scopes.last_mut().unwrap().vars.insert(name.to_string(), (addr, ty));
    }
    fn lookup_var(&self, name: &str) -> Option<(String, RType)> {
        for s in self.scopes.iter().rev() {
            if let Some(v) = s.vars.get(name) {
                return Some(v.clone());
            }
        }
        None
    }
    fn declare_const(&mut self, name: &str, cb: ConstBinding) {
        self.scopes.last_mut().unwrap().consts.insert(name.to_string(), cb);
    }
    fn lookup_const(&self, name: &str) -> Option<ConstBinding> {
        for s in self.scopes.iter().rev() {
            if let Some(c) = s.consts.get(name) {
                return Some(c.clone());
            }
        }
        None
    }
    fn lookup_function(&self, name: &str) -> Option<(String, Vec<RType>, RType, bool)> {
        if let Some(ConstBinding::Func(qn, params, ret, variadic)) = self.lookup_const(name) {
            return Some((qn, params, ret, variadic));
        }
        self.funcs.get(name).map(|f| (f.qbe_name.clone(), f.params.clone(), f.ret.clone(), f.variadic))
    }

    fn new_tmp(&mut self) -> String {
        self.tmp += 1;
        format!("%t{}", self.tmp)
    }
    fn new_label(&mut self, base: &str) -> String {
        self.lbl += 1;
        format!("@{}.{}", base, self.lbl)
    }
    fn emit(&mut self, s: &str) {
        self.cur.push_str(s);
        self.cur.push('\n');
    }
    fn emit_alloc(&mut self, size: u64, align: u64) -> String {
        let t = self.new_tmp();
        let instr = alloc_instr_for_align(align);
        let sz = size.max(1);
        let _ = writeln!(self.cur, "\t{} =l {} {}", t, instr, sz);
        t
    }
    fn emit_load(&mut self, addr: &str, ty: &RType) -> String {
        let t = self.new_tmp();
        let rt = base_ty(ty);
        let instr = load_instr(ty);
        let _ = writeln!(self.cur, "\t{} ={} {} {}", t, rt, instr, addr);
        t
    }
    fn emit_store(&mut self, addr: &str, val: &str, ty: &RType) {
        let instr = store_instr(ty);
        let _ = writeln!(self.cur, "\t{} {}, {}", instr, val, addr);
    }
    fn emit_blit(&mut self, src: &str, dst: &str, size: u64) {
        let _ = writeln!(self.cur, "\tblit {}, {}, {}", src, dst, size.max(1));
    }
    fn offset_addr(&mut self, base: &str, offset: u64) -> String {
        if offset == 0 {
            return base.to_string();
        }
        let t = self.new_tmp();
        let _ = writeln!(self.cur, "\t{} =l add {}, {}", t, base, offset);
        t
    }
    fn zero_value(&self, rt: &RType) -> String {
        match rt {
            RType::F32 => "s_0".to_string(),
            RType::F64 => "d_0".to_string(),
            _ => "0".to_string(),
        }
    }
    fn zero_memory(&mut self, addr: &str, size: u64) {
        let mut off: u64 = 0;
        while off < size {
            let remaining = size - off;
            let a = self.offset_addr(addr, off);
            if remaining >= 8 {
                self.emit_store(&a, "0", &RType::I64);
                off += 8;
            } else if remaining >= 4 {
                self.emit_store(&a, "0", &RType::I32);
                off += 4;
            } else if remaining >= 2 {
                self.emit_store(&a, "0", &RType::I16);
                off += 2;
            } else {
                self.emit_store(&a, "0", &RType::I8);
                off += 1;
            }
        }
    }


    fn emit_conv1(&mut self, instr: &str, dst_ty: &str, val: &str) -> String {
        let t = self.new_tmp();
        let _ = writeln!(self.cur, "\t{} ={} {} {}", t, dst_ty, instr, val);
        t
    }

    fn convert_scalar(&mut self, val: &str, from: &RType, to: &RType) -> String {
        if from == to {
            return val.to_string();
        }
        let from_is_f = from.is_float();
        let to_is_f = to.is_float();

        if from_is_f && to_is_f {
            return match (from, to) {
                (RType::F32, RType::F64) => self.emit_conv1("exts", "d", val),
                (RType::F64, RType::F32) => self.emit_conv1("truncd", "s", val),
                _ => val.to_string(),
            };
        }
        if !from_is_f && to_is_f {
            let from_bits = int_bits(from);
            let signed = is_signed_int(from);
            let src_is_long = from_bits > 32;
            let f_ty = if matches!(to, RType::F32) { "s" } else { "d" };
            let instr = match (src_is_long, signed) {
                (false, true) => "swtof",
                (false, false) => "uwtof",
                (true, true) => "sltof",
                (true, false) => "ultof",
            };
            return self.emit_conv1(instr, f_ty, val);
        }
        if from_is_f && !to_is_f {
            let signed = is_signed_int(to);
            let to_bits = int_bits(to);
            let f_letter = if matches!(from, RType::F32) { 's' } else { 'd' };
            let want_long = to_bits > 32;
            let instr = match (f_letter, signed) {
                ('s', true) => "stosi",
                ('s', false) => "stoui",
                ('d', true) => "dtosi",
                ('d', false) => "dtoui",
                _ => "stosi",
            };
            let dst_ty = if want_long { "l" } else { "w" };
            return self.emit_conv1(instr, dst_ty, val);
        }

        let from_bits = int_bits(from);
        let to_bits = int_bits(to);
        let from_signed = is_signed_int(from);
        let to_signed = is_signed_int(to);
        let cur_is_long = from_bits > 32;
        let mut cur_val = val.to_string();

        if to_bits <= 32 {
            if cur_is_long {
                cur_val = self.emit_conv1("copy", "w", &cur_val);
            }
            if to_bits == 8 {
                let instr = if to_signed { "extsb" } else { "extub" };
                cur_val = self.emit_conv1(instr, "w", &cur_val);
            } else if to_bits == 16 {
                let instr = if to_signed { "extsh" } else { "extuh" };
                cur_val = self.emit_conv1(instr, "w", &cur_val);
            }
            cur_val
        } else {
            if !cur_is_long {
                let instr = if from_signed { "extsw" } else { "extuw" };
                cur_val = self.emit_conv1(instr, "l", &cur_val);
            }
            cur_val
        }
    }

    fn coerce(&mut self, val: CgValue, target: &RType, line: usize, col: usize) -> Result<CgValue, CompileError> {
        let src_ty = val.ty();
        if &src_ty == target {
            return Ok(val);
        }
        match &val {
            CgValue::Scalar(s, _) if (src_ty.is_integerish() || src_ty.is_float()) && (target.is_integerish() || target.is_float()) => {
                let conv = self.convert_scalar(s, &src_ty, target);
                Ok(CgValue::Scalar(conv, target.clone()))
            }
            _ => Err(CompileError::new(
                line,
                col,
                1,
                format!("types incompatibles : attendu '{}', trouvé '{}'", type_name(target), type_name(&src_ty)),
            )),
        }
    }

    fn coerce_scalar(&mut self, val: &CgValue, target: &RType) -> Result<String, CompileError> {
        match self.coerce(val.clone(), target, 0, 0)? {
            CgValue::Scalar(s, _) => Ok(s),
            CgValue::Address(..) => Err(CompileError::new(0, 0, 1, "valeur agrégée utilisée dans un contexte scalaire")),
        }
    }

    fn unify_numeric(&mut self, lv: CgValue, rv: CgValue, line: usize, col: usize) -> Result<(RType, String, String), CompileError> {
        let (ls, lt0) = match lv {
            CgValue::Scalar(s, t) => (s, t),
            _ => return Err(CompileError::new(line, col, 1, "opérande agrégée dans une expression numérique")),
        };
        let (rs, rt0) = match rv {
            CgValue::Scalar(s, t) => (s, t),
            _ => return Err(CompileError::new(line, col, 1, "opérande agrégée dans une expression numérique")),
        };
        let lt = promote(&lt0);
        let rt = promote(&rt0);
        if !(lt.is_integerish() || lt.is_float()) || !(rt.is_integerish() || rt.is_float()) {
            return Err(CompileError::new(line, col, 1, "opérande non numérique dans une expression arithmétique/de comparaison"));
        }
        let common = if lt.is_float() || rt.is_float() {
            if matches!(lt, RType::F64) || matches!(rt, RType::F64) { RType::F64 } else { RType::F32 }
        } else if int_bits(&lt) >= 64 || int_bits(&rt) >= 64 {
            if !is_signed_int(&lt) || !is_signed_int(&rt) { RType::U64 } else { RType::I64 }
        } else if !is_signed_int(&lt) || !is_signed_int(&rt) {
            RType::U32
        } else {
            RType::I32
        };
        let lc = self.convert_scalar(&ls, &lt0, &common);
        let rc = self.convert_scalar(&rs, &rt0, &common);
        Ok((common, lc, rc))
    }

    fn to_bool_word(&mut self, v: &CgValue, line: usize, col: usize) -> Result<String, CompileError> {
        match v {
            CgValue::Scalar(s, ty) => {
                if matches!(ty, RType::Bool) {
                    return Ok(s.clone());
                }
                if !(ty.is_integerish() || ty.is_float()) {
                    return Err(CompileError::new(line, col, 1, "condition : type non convertible en booléen"));
                }
                let bty = base_ty(ty);
                let zero = match ty {
                    RType::F32 => "s_0",
                    RType::F64 => "d_0",
                    _ => "0",
                };
                let t = self.new_tmp();
                let _ = writeln!(self.cur, "\t{} =w cne{} {}, {}", t, bty, s, zero);
                Ok(t)
            }
            CgValue::Address(..) => Err(CompileError::new(line, col, 1, "une valeur agrégée ne peut pas être utilisée comme condition")),
        }
    }

    fn gen_cond(&mut self, e: &Expression) -> Result<String, CompileError> {
        let v = self.gen_expr(e)?;
        self.to_bool_word(&v, e.line, e.col)
    }


    fn gen_function(
        &mut self,
        qbe_name: &str,
        exported: bool,
        params: &[Parameter],
        variadic: bool,
        ret: RType,
        body: &Block,
    ) -> Result<(), CompileError> {
        let sret_mode = ret.is_aggregate();

        let saved_cur = std::mem::take(&mut self.cur);
        let saved_terminated = self.terminated;
        let saved_ret = std::mem::replace(&mut self.ret_type, ret.clone());
        let saved_sret = self.sret.take();
        self.terminated = false;
        self.push_scope();

        let mut sig_parts: Vec<String> = Vec::new();
        if sret_mode {
            let p = self.new_tmp();
            sig_parts.push(format!("l {}", p));
            self.sret = Some(p);
        }

        let mut prelude = String::new();
        for p in params {
            let pty = self.resolve_type(&p.ty)?;
            let incoming = format!("%.arg.{}", p.name);
            if pty.is_aggregate() {
                sig_parts.push(format!("l {}", incoming));
                self.declare_var(&p.name, incoming, pty);
            } else {
                sig_parts.push(format!("{} {}", base_ty(&pty), incoming));
                let (sz, al) = self.size_align(&pty)?;
                let instr = alloc_instr_for_align(al);
                let slot = format!("%.slot.{}", p.name);
                let _ = writeln!(prelude, "\t{} =l {} {}", slot, instr, sz.max(1));
                let _ = writeln!(prelude, "\t{} {}, {}", store_instr(&pty), incoming, slot);
                self.declare_var(&p.name, slot, pty);
            }
        }
        if variadic {
            sig_parts.push("...".to_string());
        }

        let linkage = if exported { "export " } else { "" };
        let ret_ty_str = if sret_mode || matches!(ret, RType::Void) {
            String::new()
        } else {
            format!("{} ", base_ty(&ret))
        };

        let mut header = String::new();
        let _ = writeln!(header, "{}function {}{}({}) {{", linkage, ret_ty_str, qbe_name, sig_parts.join(", "));
        header.push_str("@start\n");
        header.push_str(&prelude);
        self.cur = header;

        self.gen_block(body)?;

        if !self.terminated {
            if sret_mode || matches!(self.ret_type, RType::Void) {
                self.emit("\tret");
            } else {
                let zero = self.zero_value(&self.ret_type.clone());
                let _ = writeln!(self.cur, "\tret {}", zero);
            }
            self.terminated = true;
        }
        self.cur.push_str("}\n");
        self.functions.push(std::mem::take(&mut self.cur));

        self.pop_scope();
        self.cur = saved_cur;
        self.terminated = saved_terminated;
        self.ret_type = saved_ret;
        self.sret = saved_sret;
        Ok(())
    }

    fn hoist_local_function(
        &mut self,
        name: &str,
        params: &[Parameter],
        variadic: bool,
        decl_ty: Option<&Type>,
        fn_ret_type: Option<&Type>,
        body: &Block,
    ) -> Result<(), CompileError> {
        self.anon += 1;
        let mangled = format!("${}${}", name, self.anon);
        let param_tys = self.resolve_param_types(params)?;
        let ret = self.combined_ret_type(decl_ty, fn_ret_type)?;
        self.declare_const(name, ConstBinding::Func(mangled.clone(), param_tys, ret.clone(), variadic));
        self.gen_function(&mangled, false, params, variadic, ret, body)
    }

    fn gen_block(&mut self, b: &Block) -> Result<(), CompileError> {
        self.push_scope();
        for stmt in b {
            self.gen_stmt(stmt)?;
        }
        self.pop_scope();
        Ok(())
    }

    fn gen_stmt(&mut self, s: &Statement) -> Result<(), CompileError> {
        match &s.kind {
            StmtKind::Block(b) => self.gen_block(b),
            StmtKind::LocalVarDecl(decl) => self.gen_local_var_decl(decl),
            StmtKind::If(ifs) => self.gen_if(ifs),
            StmtKind::While(w) => self.gen_while(w),
            StmtKind::For(f) => self.gen_for(f),
            StmtKind::Return(opt) => self.gen_return(opt.as_ref(), s.line, s.col),
            StmtKind::Expr(e) => {
                self.gen_expr(e)?;
                Ok(())
            }
        }
    }

    fn store_new_local(&mut self, name: &str, ty: &RType, val: &CgValue) -> Result<(), CompileError> {
        let (sz, al) = self.size_align(ty)?;
        let slot = self.emit_alloc(sz, al);
        if ty.is_aggregate() {
            match val {
                CgValue::Address(a, _) => self.emit_blit(a, &slot, sz),
                _ => return Err(CompileError::new(0, 0, 1, "valeur scalaire assignée à une variable de type agrégé")),
            }
        } else {
            let scalar = self.coerce_scalar(val, ty)?;
            self.emit_store(&slot, &scalar, ty);
        }
        self.declare_var(name, slot, ty.clone());
        Ok(())
    }

    fn gen_local_var_decl(&mut self, decl: &LocalVarDecl) -> Result<(), CompileError> {
        match decl {
            LocalVarDecl::Mutable { ty, name, value } => {
                let declared = match ty {
                    Some(t) => Some(self.resolve_type(t)?),
                    None => None,
                };
                let val = self.gen_expr_as(value, declared.as_ref())?;
                let final_ty = declared.unwrap_or_else(|| val.ty());
                self.store_new_local(name, &final_ty, &val)
            }
            LocalVarDecl::Constant { ty, name, value } => match value {
                BindingValue::Function(fd) => {
                    let body = fd.body.as_ref().ok_or_else(|| {
                        CompileError::new(0, 0, name.chars().count().max(1), format!(
                            "la fonction locale '{}' doit avoir un corps ('{{ ... }}' ou '=> expression') ; '@extern' n'est pas supporté pour les fonctions imbriquées",
                            name
                        ))
                    })?;
                    self.hoist_local_function(name, &fd.params, fd.variadic, ty.as_ref(), fd.ret_type.as_ref(), body)
                }
                BindingValue::ExpressionFunction(efd) => {
                    let ret_stmt = Statement {
                        kind: StmtKind::Return(Some((*efd.body).clone())),
                        line: 0,
                        col: 0,
                    };
                    let body = vec![ret_stmt];
                    self.hoist_local_function(name, &efd.params, efd.variadic, ty.as_ref(), efd.ret_type.as_ref(), &body)
                }
                BindingValue::Struct(sd) => {
                    self.anon += 1;
                    let mangled = format!("{}${}", name, self.anon);
                    self.resolve_struct_layout(&mangled, &sd.fields)?;
                    self.declare_const(name, ConstBinding::TypeAlias(mangled));
                    Ok(())
                }
                BindingValue::Expr(e) => {
                    let declared = match ty {
                        Some(t) => Some(self.resolve_type(t)?),
                        None => None,
                    };
                    let val = self.gen_expr_as(e, declared.as_ref())?;
                    let final_ty = declared.unwrap_or_else(|| val.ty());
                    self.store_new_local(name, &final_ty, &val)
                }
            },
        }
    }

    fn gen_if(&mut self, ifs: &IfStmt) -> Result<(), CompileError> {
        let cond = self.gen_cond(&ifs.cond)?;
        let then_lbl = self.new_label("if.then");
        let else_lbl = self.new_label("if.else");
        let end_lbl = self.new_label("if.end");
        let has_else = ifs.else_branch.is_some();
        let false_target = if has_else { else_lbl.clone() } else { end_lbl.clone() };
        let _ = writeln!(self.cur, "\tjnz {}, {}, {}", cond, then_lbl, false_target);
        self.terminated = true;

        let _ = writeln!(self.cur, "{}", then_lbl);
        self.terminated = false;
        self.gen_block(&ifs.then_block)?;
        let then_falls_through = !self.terminated;
        if then_falls_through {
            let _ = writeln!(self.cur, "\tjmp {}", end_lbl);
        }
        self.terminated = true;

        let mut else_falls_through = false;
        if let Some(branch) = &ifs.else_branch {
            let _ = writeln!(self.cur, "{}", else_lbl);
            self.terminated = false;
            match branch {
                ElseBranch::Block(b) => self.gen_block(b)?,
                ElseBranch::If(inner) => self.gen_if(inner)?,
            }
            else_falls_through = !self.terminated;
            if else_falls_through {
                let _ = writeln!(self.cur, "\tjmp {}", end_lbl);
            }
            self.terminated = true;
        }

        let end_reachable = then_falls_through || !has_else || else_falls_through;
        let _ = writeln!(self.cur, "{}", end_lbl);
        self.terminated = !end_reachable;
        Ok(())
    }

    fn gen_while(&mut self, w: &WhileStmt) -> Result<(), CompileError> {
        let cond_lbl = self.new_label("while.cond");
        let body_lbl = self.new_label("while.body");
        let end_lbl = self.new_label("while.end");

        if !self.terminated {
            let _ = writeln!(self.cur, "\tjmp {}", cond_lbl);
        }
        self.terminated = true;
        let _ = writeln!(self.cur, "{}", cond_lbl);
        self.terminated = false;
        let cond = self.gen_cond(&w.cond)?;
        let _ = writeln!(self.cur, "\tjnz {}, {}, {}", cond, body_lbl, end_lbl);
        self.terminated = true;

        let _ = writeln!(self.cur, "{}", body_lbl);
        self.terminated = false;
        self.gen_block(&w.body)?;
        if !self.terminated {
            let _ = writeln!(self.cur, "\tjmp {}", cond_lbl);
        }
        self.terminated = true;

        let _ = writeln!(self.cur, "{}", end_lbl);
        self.terminated = false;
        Ok(())
    }

    fn gen_for(&mut self, f: &ForStmt) -> Result<(), CompileError> {
        self.push_scope();
        if let Some(init) = &f.init {
            self.gen_local_var_decl(init)?;
        }
        let cond_lbl = self.new_label("for.cond");
        let body_lbl = self.new_label("for.body");
        let end_lbl = self.new_label("for.end");

        if !self.terminated {
            let _ = writeln!(self.cur, "\tjmp {}", cond_lbl);
        }
        self.terminated = true;
        let _ = writeln!(self.cur, "{}", cond_lbl);
        self.terminated = false;
        if let Some(c) = &f.cond {
            let cond = self.gen_cond(c)?;
            let _ = writeln!(self.cur, "\tjnz {}, {}, {}", cond, body_lbl, end_lbl);
        } else {
            let _ = writeln!(self.cur, "\tjmp {}", body_lbl);
        }
        self.terminated = true;

        let _ = writeln!(self.cur, "{}", body_lbl);
        self.terminated = false;
        self.gen_block(&f.body)?;
        if !self.terminated {
            if let Some(post) = &f.post {
                self.gen_expr(post)?;
            }
            let _ = writeln!(self.cur, "\tjmp {}", cond_lbl);
        }
        self.terminated = true;

        let _ = writeln!(self.cur, "{}", end_lbl);
        self.terminated = false;
        self.pop_scope();
        Ok(())
    }

    fn gen_return(&mut self, e: Option<&Expression>, line: usize, col: usize) -> Result<(), CompileError> {
        let ret_ty = self.ret_type.clone();
        match e {
            None => {
                if !matches!(ret_ty, RType::Void) {
                    return Err(CompileError::new(line, col, 3, "un 'ret' sans valeur est utilisé dans une fonction qui doit renvoyer une valeur"));
                }
                self.emit("\tret");
            }
            Some(expr) => {
                if matches!(ret_ty, RType::Void) {
                    return Err(CompileError::new(expr.line, expr.col, 1, "cette fonction ne doit renvoyer aucune valeur ('ret' sans expression attendu)"));
                }
                if ret_ty.is_aggregate() {
                    let val = self.gen_expr_as(expr, Some(&ret_ty))?;
                    let sret = self.sret.clone().expect("sret attendu pour un retour agrégé");
                    match val {
                        CgValue::Address(a, _) => {
                            let (sz, _al) = self.size_align(&ret_ty)?;
                            self.emit_blit(&a, &sret, sz);
                        }
                        _ => return Err(CompileError::new(expr.line, expr.col, 1, "valeur agrégée attendue pour ce 'ret'")),
                    }
                    self.emit("\tret");
                } else {
                    let val = self.gen_expr_as(expr, Some(&ret_ty))?;
                    let scalar = self.coerce_scalar(&val, &ret_ty)?;
                    let _ = writeln!(self.cur, "\tret {}", scalar);
                }
            }
        }
        self.terminated = true;
        Ok(())
    }

    fn gen_lvalue(&mut self, e: &Expression) -> Result<(String, RType), CompileError> {
        match &e.kind {
            ExprKind::Identifier(name) => {
                if let Some((addr, ty)) = self.lookup_var(name) {
                    return Ok((addr, ty));
                }
                if let Some((sym, ty)) = self.global_consts.get(name).cloned() {
                    return Ok((sym, ty));
                }
                Err(CompileError::new(e.line, e.col, name.chars().count().max(1), format!("variable inconnue : '{}'", name)))
            }
            ExprKind::Unary { op, expr } if op == "*" => {
                let v = self.gen_expr(expr)?;
                let (pval, pty) = match v {
                    CgValue::Scalar(s, t) => (s, t),
                    CgValue::Address(s, t) => (s, t),
                };
                match pty {
                    RType::Pointer(inner) => Ok((pval, *inner)),
                    _ => Err(CompileError::new(e.line, e.col, 1, format!("impossible de déréférencer une valeur de type '{}'", type_name(&pty)))),
                }
            }
            ExprKind::Index { base, index } => self.gen_index_addr(base, index, e.line, e.col),
            ExprKind::Member { base, member } => self.gen_member_addr(base, member, e.line, e.col),
            _ => Err(CompileError::new(e.line, e.col, 1, "cette expression n'est pas assignable (pas une lvalue)")),
        }
    }

    fn gen_index_addr(&mut self, base: &Expression, index: &Expression, line: usize, col: usize) -> Result<(String, RType), CompileError> {
        let base_val = self.gen_expr(base)?;
        let (base_ptr, elem_ty): (String, RType) = match base_val.ty() {
            RType::Array(_n, elem) => {
                let addr = match &base_val {
                    CgValue::Address(a, _) => a.clone(),
                    _ => return Err(CompileError::new(line, col, 1, "tableau attendu")),
                };
                (addr, *elem)
            }
            RType::Slice(elem) => {
                let addr = match &base_val {
                    CgValue::Address(a, _) => a.clone(),
                    _ => return Err(CompileError::new(line, col, 1, "slice attendue")),
                };
                let ptr = self.emit_load(&addr, &RType::RawPtr);
                (ptr, *elem)
            }
            RType::Pointer(elem) => {
                let ptr_ty = RType::Pointer(elem.clone());
                let ptr = self.coerce_scalar(&base_val, &ptr_ty)?;
                (ptr, *elem)
            }
            other => return Err(CompileError::new(line, col, 1, format!("impossible d'indexer une valeur de type '{}'", type_name(&other)))),
        };
        let idx = self.gen_expr_as(index, Some(&RType::I64))?;
        let idx_val = self.coerce_scalar(&idx, &RType::I64)?;
        let (esz, _eal) = self.size_align(&elem_ty)?;
        let off = self.new_tmp();
        let _ = writeln!(self.cur, "\t{} =l mul {}, {}", off, idx_val, esz);
        let addr = self.new_tmp();
        let _ = writeln!(self.cur, "\t{} =l add {}, {}", addr, base_ptr, off);
        Ok((addr, elem_ty))
    }

    fn gen_member_addr(&mut self, base: &Expression, member: &str, line: usize, col: usize) -> Result<(String, RType), CompileError> {
        let base_val = self.gen_expr(base)?;
        let base_ty = base_val.ty();
        let (struct_addr, struct_name): (String, String) = match &base_ty {
            RType::Struct(n) => {
                let a = match &base_val {
                    CgValue::Address(a, _) => a.clone(),
                    _ => return Err(CompileError::new(line, col, 1, "structure attendue")),
                };
                (a, n.clone())
            }
            RType::Pointer(inner) => match inner.as_ref() {
                RType::Struct(n) => {
                    let p = self.coerce_scalar(&base_val, &base_ty)?;
                    (p, n.clone())
                }
                _ => return Err(CompileError::new(line, col, 1, "accès de membre sur un pointeur qui ne pointe pas vers une structure")),
            },
            RType::Slice(elem) => {
                let addr = match &base_val {
                    CgValue::Address(a, _) => a.clone(),
                    _ => return Err(CompileError::new(line, col, 1, "slice attendue")),
                };
                return match member {
                    "ptr" => Ok((addr, RType::Pointer(elem.clone()))),
                    "len" => {
                        let a2 = self.offset_addr(&addr, 8);
                        Ok((a2, RType::I64))
                    }
                    _ => Err(CompileError::new(line, col, member.chars().count().max(1), format!("le type slice n'a pas de champ '{}'", member))),
                };
            }
            other => return Err(CompileError::new(line, col, 1, format!("accès de membre sur un type non-structure : '{}'", type_name(other)))),
        };
        let layout = self
            .structs
            .get(&struct_name)
            .and_then(|o| o.clone())
            .ok_or_else(|| CompileError::new(line, col, 1, format!("type de structure inconnu ou incomplet : '{}'", struct_name)))?;
        let field = layout
            .fields
            .iter()
            .find(|(n, _, _)| n == member)
            .cloned()
            .ok_or_else(|| CompileError::new(line, col, member.chars().count().max(1), format!("la structure '{}' n'a pas de champ '{}'", struct_name, member)))?;
        let addr = self.offset_addr(&struct_addr, field.2);
        Ok((addr, field.1))
    }

    fn load_or_address(&mut self, addr: String, ty: RType) -> Result<CgValue, CompileError> {
        if ty.is_aggregate() {
            Ok(CgValue::Address(addr, ty))
        } else {
            let t = self.emit_load(&addr, &ty);
            Ok(CgValue::Scalar(t, ty))
        }
    }

    fn store_value(&mut self, addr: &str, val: &CgValue, ty: &RType) -> Result<(), CompileError> {
        if ty.is_aggregate() {
            match val {
                CgValue::Address(a, _) => {
                    let (sz, _al) = self.size_align(ty)?;
                    self.emit_blit(a, addr, sz);
                    Ok(())
                }
                _ => Err(CompileError::new(0, 0, 1, "valeur scalaire assignée à un emplacement de type agrégé")),
            }
        } else {
            let s = self.coerce_scalar(val, ty)?;
            self.emit_store(addr, &s, ty);
            Ok(())
        }
    }

    fn gen_identifier(&mut self, name: &str, line: usize, col: usize) -> Result<CgValue, CompileError> {
        if let Some((addr, ty)) = self.lookup_var(name) {
            return self.load_or_address(addr, ty);
        }
        if let Some((sym, ty)) = self.global_consts.get(name).cloned() {
            return self.load_or_address(sym, ty);
        }
        if self.lookup_function(name).is_some() {
            return Err(CompileError::new(
                line,
                col,
                name.chars().count().max(1),
                "les fonctions ne sont pas des valeurs de première classe dans ce générateur : elles ne peuvent être qu'appelées directement ('f(...)'), pas manipulées comme des pointeurs",
            ));
        }
        Err(CompileError::new(line, col, name.chars().count().max(1), format!("identifiant inconnu : '{}'", name)))
    }

    fn gen_expr(&mut self, e: &Expression) -> Result<CgValue, CompileError> {
        match &e.kind {
            ExprKind::Identifier(name) => self.gen_identifier(name, e.line, e.col),
            ExprKind::IntLiteral(v) => Ok(CgValue::Scalar(v.to_string(), RType::I64)),
            ExprKind::FloatLiteral(v) => Ok(CgValue::Scalar(format!("d_{}", v), RType::F64)),
            ExprKind::StringLiteral(s) => {
                let sym = self.intern_string(s);
                Ok(CgValue::Scalar(sym, RType::Pointer(Box::new(RType::U8))))
            }
            ExprKind::CharLiteral(c) => Ok(CgValue::Scalar(((*c as u32) & 0xFF).to_string(), RType::U8)),
            ExprKind::BoolLiteral(b) => Ok(CgValue::Scalar(if *b { "1".to_string() } else { "0".to_string() }, RType::Bool)),
            ExprKind::Null => Ok(CgValue::Scalar("0".to_string(), RType::RawPtr)),
            ExprKind::StructInit { name, fields } => self.gen_struct_init(name, fields, e.line, e.col),
            ExprKind::Cast { ty, expr } => self.gen_cast(ty, expr, e.line, e.col),
            ExprKind::Unary { op, expr } => self.gen_unary(op, expr, e.line, e.col),
            ExprKind::Binary { op, left, right } => self.gen_binary(op, left, right, e.line, e.col),
            ExprKind::Assign { op, target, value } => self.gen_assign(op, target, value, e.line, e.col),
            ExprKind::Call { callee, args } => self.gen_call(callee, args, e.line, e.col),
            ExprKind::Index { base, index } => {
                let (a, t) = self.gen_index_addr(base, index, e.line, e.col)?;
                self.load_or_address(a, t)
            }
            ExprKind::Member { base, member } => {
                let (a, t) = self.gen_member_addr(base, member, e.line, e.col)?;
                self.load_or_address(a, t)
            }
            ExprKind::Slice { base, start, end } => self.gen_slice(base, start.as_deref(), end.as_deref(), e.line, e.col),
            ExprKind::VaStart => self.gen_va_start(),
            ExprKind::VaArg { list, ty } => self.gen_va_arg(list, ty, e.line, e.col),
            ExprKind::VaEnd(list) => self.gen_va_end(list),
        }
    }

    fn gen_va_start(&mut self) -> Result<CgValue, CompileError> {
        let buf = self.emit_alloc(32, 8);
        let _ = writeln!(self.cur, "\tvastart {}", buf);
        Ok(CgValue::Scalar(buf, RType::RawPtr))
    }

    fn gen_va_arg(&mut self, list: &Expression, ty: &Type, line: usize, col: usize) -> Result<CgValue, CompileError> {
        let list_val = self.gen_expr(list)?;
        let list_ptr = self.coerce_scalar(&list_val, &RType::RawPtr)?;
        let vty = self.resolve_type(ty)?;
        if vty.is_aggregate() {
            return Err(CompileError::new(
                line,
                col,
                1,
                "'va_arg' ne peut lire que des types de base (entiers, flottants, pointeurs) : QBE ne sait pas lire une structure/un tableau/une slice depuis une liste d'arguments variables",
            ));
        }
        if matches!(vty, RType::I8 | RType::I16 | RType::U8 | RType::U16 | RType::Bool | RType::F32) {
            return Err(CompileError::new(
                line,
                col,
                1,
                format!(
                    "'va_arg' avec le type '{}' est probablement une erreur : les arguments variables subissent une promotion par défaut (comme en C) — demandez 'i32'/'u32' pour un entier étroit, ou 'f64' à la place de 'f32'",
                    type_name(&vty)
                ),
            ));
        }
        let t = self.new_tmp();
        let _ = writeln!(self.cur, "\t{} ={} vaarg {}", t, base_ty(&vty), list_ptr);
        Ok(CgValue::Scalar(t, vty))
    }

    fn gen_va_end(&mut self, list: &Expression) -> Result<CgValue, CompileError> {
        self.gen_expr(list)?;
        Ok(CgValue::Scalar("0".to_string(), RType::Void))
    }

    fn gen_expr_as(&mut self, e: &Expression, expected: Option<&RType>) -> Result<CgValue, CompileError> {
        match (&e.kind, expected) {
            (ExprKind::IntLiteral(v), Some(t)) if t.is_integerish() => Ok(CgValue::Scalar(v.to_string(), t.clone())),
            (ExprKind::IntLiteral(v), Some(RType::F32)) => Ok(CgValue::Scalar(format!("s_{}", *v as f64), RType::F32)),
            (ExprKind::IntLiteral(v), Some(RType::F64)) => Ok(CgValue::Scalar(format!("d_{}", *v as f64), RType::F64)),
            (ExprKind::FloatLiteral(v), Some(RType::F32)) => Ok(CgValue::Scalar(format!("s_{}", v), RType::F32)),
            (ExprKind::Null, Some(t)) if matches!(t, RType::RawPtr | RType::Pointer(_)) => Ok(CgValue::Scalar("0".to_string(), t.clone())),
            _ => {
                let val = self.gen_expr(e)?;
                match expected {
                    Some(t) => self.coerce(val, t, e.line, e.col),
                    None => Ok(val),
                }
            }
        }
    }

    fn gen_struct_init(&mut self, name: &str, fields: &[(String, Expression)], line: usize, col: usize) -> Result<CgValue, CompileError> {
        let struct_name = self
            .resolve_custom_name(name)
            .ok_or_else(|| CompileError::new(line, col, name.chars().count().max(1), format!("type de structure inconnu : '{}'", name)))?;
        let layout = self
            .structs
            .get(&struct_name)
            .and_then(|o| o.clone())
            .ok_or_else(|| CompileError::new(line, col, 1, format!("type '{}' incomplet ou récursif", struct_name)))?;
        let addr = self.emit_alloc(layout.size, layout.align);
        self.zero_memory(&addr, layout.size);
        for (fname, fexpr) in fields {
            let field = layout
                .fields
                .iter()
                .find(|(n, _, _)| n == fname)
                .cloned()
                .ok_or_else(|| CompileError::new(fexpr.line, fexpr.col, fname.chars().count().max(1), format!("la structure '{}' n'a pas de champ '{}'", struct_name, fname)))?;
            let faddr = self.offset_addr(&addr, field.2);
            let val = self.gen_expr_as(fexpr, Some(&field.1))?;
            self.store_value(&faddr, &val, &field.1)?;
        }
        Ok(CgValue::Address(addr, RType::Struct(struct_name)))
    }

    fn gen_cast(&mut self, ty: &Type, expr: &Expression, line: usize, col: usize) -> Result<CgValue, CompileError> {
        let target = self.resolve_type(ty)?;
        if target.is_aggregate() {
            return Err(CompileError::new(line, col, 1, "les conversions ('cast') vers un type agrégé (structure, tableau, slice) ne sont pas supportées"));
        }
        let val = self.gen_expr(expr)?;
        match val {
            CgValue::Scalar(s, from) => {
                let conv = self.convert_scalar(&s, &from, &target);
                Ok(CgValue::Scalar(conv, target))
            }
            CgValue::Address(..) => Err(CompileError::new(line, col, 1, "impossible de convertir une valeur agrégée avec 'cast' ; utilisez '&' pour obtenir un pointeur si besoin")),
        }
    }

    fn gen_unary(&mut self, op: &str, expr: &Expression, line: usize, col: usize) -> Result<CgValue, CompileError> {
        match op {
            "-" => {
                let v = self.gen_expr(expr)?;
                match v {
                    CgValue::Scalar(s, ty) => {
                        if !(ty.is_integerish() || ty.is_float()) {
                            return Err(CompileError::new(line, col, 1, "opérateur unaire '-' : type numérique attendu"));
                        }
                        let t = self.new_tmp();
                        let _ = writeln!(self.cur, "\t{} ={} neg {}", t, base_ty(&ty), s);
                        Ok(CgValue::Scalar(t, ty))
                    }
                    CgValue::Address(..) => Err(CompileError::new(line, col, 1, "opérateur unaire '-' : type numérique attendu")),
                }
            }
            "!" => {
                let v = self.gen_expr(expr)?;
                let cond = self.to_bool_word(&v, line, col)?;
                let t = self.new_tmp();
                let _ = writeln!(self.cur, "\t{} =w ceqw {}, 0", t, cond);
                Ok(CgValue::Scalar(t, RType::Bool))
            }
            "&" => {
                let (addr, ty) = self.gen_lvalue(expr)?;
                Ok(CgValue::Scalar(addr, RType::Pointer(Box::new(ty))))
            }
            "*" => {
                let v = self.gen_expr(expr)?;
                let (pval, pty) = match v {
                    CgValue::Scalar(s, t) => (s, t),
                    CgValue::Address(s, t) => (s, t),
                };
                match pty {
                    RType::Pointer(inner) => self.load_or_address(pval, *inner),
                    _ => Err(CompileError::new(line, col, 1, format!("impossible de déréférencer une valeur de type '{}'", type_name(&pty)))),
                }
            }
            other => Err(CompileError::new(line, col, other.chars().count().max(1), format!("opérateur unaire inconnu : '{}'", other))),
        }
    }

    fn gen_logical(&mut self, op: &str, left: &Expression, right: &Expression, _line: usize, _col: usize) -> Result<CgValue, CompileError> {
        let result_slot = self.emit_alloc(1, 1);
        let lcond = self.gen_cond(left)?;
        let rhs_lbl = self.new_label(if op == "&&" { "and.rhs" } else { "or.rhs" });
        let short_lbl = self.new_label(if op == "&&" { "and.short" } else { "or.short" });
        let end_lbl = self.new_label(if op == "&&" { "and.end" } else { "or.end" });

        if op == "&&" {
            let _ = writeln!(self.cur, "\tjnz {}, {}, {}", lcond, rhs_lbl, short_lbl);
        } else {
            let _ = writeln!(self.cur, "\tjnz {}, {}, {}", lcond, short_lbl, rhs_lbl);
        }
        self.terminated = true;

        let _ = writeln!(self.cur, "{}", rhs_lbl);
        self.terminated = false;
        let rcond = self.gen_cond(right)?;
        self.emit_store(&result_slot, &rcond, &RType::Bool);
        let _ = writeln!(self.cur, "\tjmp {}", end_lbl);
        self.terminated = true;

        let _ = writeln!(self.cur, "{}", short_lbl);
        self.terminated = false;
        let shortval = if op == "&&" { "0" } else { "1" };
        self.emit_store(&result_slot, shortval, &RType::Bool);
        let _ = writeln!(self.cur, "\tjmp {}", end_lbl);
        self.terminated = true;

        let _ = writeln!(self.cur, "{}", end_lbl);
        self.terminated = false;
        let t = self.emit_load(&result_slot, &RType::Bool);
        Ok(CgValue::Scalar(t, RType::Bool))
    }

    fn gen_binary(&mut self, op: &str, left: &Expression, right: &Expression, line: usize, col: usize) -> Result<CgValue, CompileError> {
        if op == "&&" || op == "||" {
            return self.gen_logical(op, left, right, line, col);
        }
        let lv = self.gen_expr(left)?;
        let rv = self.gen_expr(right)?;
        let (common, lc, rc) = self.unify_numeric(lv, rv, line, col)?;
        match op {
            "+" | "-" | "*" | "/" => {
                let instr = match op {
                    "+" => "add",
                    "-" => "sub",
                    "*" => "mul",
                    "/" => {
                        if common.is_float() {
                            "div"
                        } else if is_signed_int(&common) {
                            "div"
                        } else {
                            "udiv"
                        }
                    }
                    _ => unreachable!(),
                };
                let t = self.new_tmp();
                let _ = writeln!(self.cur, "\t{} ={} {} {}, {}", t, base_ty(&common), instr, lc, rc);
                Ok(CgValue::Scalar(t, common))
            }
            "%" => {
                if common.is_float() {
                    return Err(CompileError::new(line, col, 1, "l'opérateur '%' n'est pas défini pour les flottants"));
                }
                let instr = if is_signed_int(&common) { "rem" } else { "urem" };
                let t = self.new_tmp();
                let _ = writeln!(self.cur, "\t{} ={} {} {}, {}", t, base_ty(&common), instr, lc, rc);
                Ok(CgValue::Scalar(t, common))
            }
            "==" | "!=" | "<" | "<=" | ">" | ">=" => {
                let t = self.new_tmp();
                let instr = cmp_instr(op, &common);
                let _ = writeln!(self.cur, "\t{} =w {} {}, {}", t, instr, lc, rc);
                Ok(CgValue::Scalar(t, RType::Bool))
            }
            other => Err(CompileError::new(line, col, other.chars().count().max(1), format!("opérateur binaire inconnu : '{}'", other))),
        }
    }

    fn gen_assign(&mut self, op: &str, target: &Expression, value: &Expression, line: usize, col: usize) -> Result<CgValue, CompileError> {
        let (addr, ty) = self.gen_lvalue(target)?;
        if op == "=" {
            let val = self.gen_expr_as(value, Some(&ty))?;
            self.store_value(&addr, &val, &ty)?;
            return self.load_or_address(addr, ty);
        }
        if ty.is_aggregate() {
            return Err(CompileError::new(line, col, op.chars().count().max(1), "les opérateurs d'affectation composée ('+=', etc.) ne sont pas supportés sur les types agrégés"));
        }
        let cur_val = CgValue::Scalar(self.emit_load(&addr, &ty), ty.clone());
        let rhs = self.gen_expr(value)?;
        let (common, lc, rc) = self.unify_numeric(cur_val, rhs, line, col)?;
        let instr = match op {
            "+=" => "add",
            "-=" => "sub",
            "*=" => "mul",
            "/=" => {
                if common.is_float() {
                    "div"
                } else if is_signed_int(&common) {
                    "div"
                } else {
                    "udiv"
                }
            }
            other => return Err(CompileError::new(line, col, other.chars().count().max(1), format!("opérateur d'affectation inconnu : '{}'", other))),
        };
        let t = self.new_tmp();
        let _ = writeln!(self.cur, "\t{} ={} {} {}, {}", t, base_ty(&common), instr, lc, rc);
        let result = CgValue::Scalar(t, common);
        let back = self.coerce_scalar(&result, &ty)?;
        self.emit_store(&addr, &back, &ty);
        Ok(CgValue::Scalar(back, ty))
    }

    fn gen_call(&mut self, callee: &Expression, args: &[Expression], line: usize, col: usize) -> Result<CgValue, CompileError> {
        let name = match &callee.kind {
            ExprKind::Identifier(n) => n.clone(),
            _ => return Err(CompileError::new(callee.line, callee.col, 1, "seuls les appels de fonctions nommées directement sont supportés (pas de pointeurs de fonction dans ce générateur)")),
        };
        let (qbe_name, params, ret, variadic) = self
            .lookup_function(&name)
            .ok_or_else(|| CompileError::new(callee.line, callee.col, name.chars().count().max(1), format!("fonction inconnue : '{}'", name)))?;
        if variadic && args.len() < params.len() {
            return Err(CompileError::new(line, col, 1, format!("'{}' attend au moins {} argument(s), {} fourni(s)", name, params.len(), args.len())));
        }
        if !variadic && args.len() != params.len() {
            return Err(CompileError::new(line, col, 1, format!("'{}' attend {} argument(s), {} fourni(s)", name, params.len(), args.len())));
        }

        let sret_mode = ret.is_aggregate();
        let mut sret_slot: Option<String> = None;
        let mut arg_parts: Vec<String> = Vec::new();
        if sret_mode {
            let (sz, al) = self.size_align(&ret)?;
            let slot = self.emit_alloc(sz, al);
            arg_parts.push(format!("l {}", slot));
            sret_slot = Some(slot);
        }
        for (arg, pty) in args.iter().zip(params.iter()) {
            let val = self.gen_expr_as(arg, Some(pty))?;
            if pty.is_aggregate() {
                let src = match &val {
                    CgValue::Address(a, _) => a.clone(),
                    _ => return Err(CompileError::new(arg.line, arg.col, 1, "argument agrégé attendu")),
                };
                let (sz, al) = self.size_align(pty)?;
                let copy_slot = self.emit_alloc(sz, al);
                self.emit_blit(&src, &copy_slot, sz);
                arg_parts.push(format!("l {}", copy_slot));
            } else {
                let s = self.coerce_scalar(&val, pty)?;
                arg_parts.push(format!("{} {}", base_ty(pty), s));
            }
        }
        if variadic {
            arg_parts.push("...".to_string());
            for arg in &args[params.len()..] {
                let val = self.gen_expr(arg)?;
                let natural_ty = val.ty();
                if natural_ty.is_aggregate() {
                    return Err(CompileError::new(
                        arg.line,
                        arg.col,
                        1,
                        "les arguments agrégés (structure, tableau, slice) ne sont pas supportés dans la partie variable d'un appel ('...') ; passez un pointeur explicite",
                    ));
                }
                let promoted = variadic_promote(&natural_ty);
                let s = self.coerce_scalar(&val, &promoted)?;
                arg_parts.push(format!("{} {}", base_ty(&promoted), s));
            }
        }
        let args_str = arg_parts.join(", ");

        if sret_mode {
            let _ = writeln!(self.cur, "\tcall {}({})", qbe_name, args_str);
            Ok(CgValue::Address(sret_slot.unwrap(), ret))
        } else if matches!(ret, RType::Void) {
            let _ = writeln!(self.cur, "\tcall {}({})", qbe_name, args_str);
            Ok(CgValue::Scalar("0".to_string(), RType::Void))
        } else {
            let t = self.new_tmp();
            let _ = writeln!(self.cur, "\t{} ={} call {}({})", t, base_ty(&ret), qbe_name, args_str);
            Ok(CgValue::Scalar(t, ret))
        }
    }

    fn gen_slice(&mut self, base: &Expression, start: Option<&Expression>, end: Option<&Expression>, line: usize, col: usize) -> Result<CgValue, CompileError> {
        let base_val = self.gen_expr(base)?;
        let (base_ptr, elem_ty, len_default): (String, RType, Option<String>) = match base_val.ty() {
            RType::Array(n, elem) => {
                let addr = match &base_val {
                    CgValue::Address(a, _) => a.clone(),
                    _ => return Err(CompileError::new(line, col, 1, "tableau attendu")),
                };
                (addr, *elem, Some(n.to_string()))
            }
            RType::Slice(elem) => {
                let addr = match &base_val {
                    CgValue::Address(a, _) => a.clone(),
                    _ => return Err(CompileError::new(line, col, 1, "slice attendue")),
                };
                let ptr = self.emit_load(&addr, &RType::RawPtr);
                let len_addr = self.offset_addr(&addr, 8);
                let len = self.emit_load(&len_addr, &RType::I64);
                (ptr, *elem, Some(len))
            }
            RType::Pointer(elem) => {
                let ptr_ty = RType::Pointer(elem.clone());
                let ptr = self.coerce_scalar(&base_val, &ptr_ty)?;
                (ptr, *elem, None)
            }
            other => return Err(CompileError::new(line, col, 1, format!("impossible de découper (slice) une valeur de type '{}'", type_name(&other)))),
        };

        let start_val: String = match start {
            Some(s) => {
                let v = self.gen_expr_as(s, Some(&RType::I64))?;
                self.coerce_scalar(&v, &RType::I64)?
            }
            None => "0".to_string(),
        };
        let end_val: String = match end {
            Some(en) => {
                let v = self.gen_expr_as(en, Some(&RType::I64))?;
                self.coerce_scalar(&v, &RType::I64)?
            }
            None => len_default.ok_or_else(|| {
                CompileError::new(line, col, 1, "borne de fin manquante : impossible de déterminer la longueur implicite pour ce type de base")
            })?,
        };

        let (esz, _eal) = self.size_align(&elem_ty)?;
        let off = self.new_tmp();
        let _ = writeln!(self.cur, "\t{} =l mul {}, {}", off, start_val, esz);
        let new_ptr = self.new_tmp();
        let _ = writeln!(self.cur, "\t{} =l add {}, {}", new_ptr, base_ptr, off);
        let new_len = self.new_tmp();
        let _ = writeln!(self.cur, "\t{} =l sub {}, {}", new_len, end_val, start_val);

        let slot = self.emit_alloc(16, 8);
        self.emit_store(&slot, &new_ptr, &RType::RawPtr);
        let len_slot = self.offset_addr(&slot, 8);
        self.emit_store(&len_slot, &new_len, &RType::I64);
        Ok(CgValue::Address(slot, RType::Slice(Box::new(elem_ty))))
    }
}

fn builtin_rtype(s: &str) -> Option<RType> {
    Some(match s {
        "i8" => RType::I8,
        "i16" => RType::I16,
        "i32" => RType::I32,
        "i64" => RType::I64,
        "u8" => RType::U8,
        "u16" => RType::U16,
        "u32" => RType::U32,
        "u64" => RType::U64,
        "f32" => RType::F32,
        "f64" => RType::F64,
        "bool" => RType::Bool,
        "rawptr" => RType::RawPtr,
        _ => return None,
    })
}