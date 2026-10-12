//! Semantic types shared by the type checker and the code generator.

#[derive(Debug, Clone, PartialEq)]
pub enum RType {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Bool,
    RawPtr,
    Pointer(Box<RType>),
    Slice(Box<RType>),
    Array(u64, Box<RType>),
    Struct(String),
    /// Function pointer: parameter types and return type.
    Func(Vec<RType>, Box<RType>),
    Void,
    /// Placeholder used by the type checker once an error has been reported,
    /// so that one mistake does not produce a cascade of follow-up errors.
    Error,
}

impl RType {
    pub fn is_aggregate(&self) -> bool {
        matches!(self, RType::Struct(_) | RType::Array(_, _) | RType::Slice(_))
    }

    pub fn is_float(&self) -> bool {
        matches!(self, RType::F32 | RType::F64)
    }

    /// Anything that lives in an integer register (integers, `bool`, pointers).
    pub fn is_integerish(&self) -> bool {
        matches!(
            self,
            RType::I8 | RType::I16 | RType::I32 | RType::I64
                | RType::U8 | RType::U16 | RType::U32 | RType::U64
                | RType::Bool | RType::RawPtr | RType::Pointer(_) | RType::Func(_, _)
        )
    }

    /// Only the integer types `i8..u64`.
    pub fn is_int(&self) -> bool {
        matches!(
            self,
            RType::I8 | RType::I16 | RType::I32 | RType::I64 | RType::U8 | RType::U16 | RType::U32 | RType::U64
        )
    }

    pub fn is_signed_int(&self) -> bool {
        matches!(self, RType::I8 | RType::I16 | RType::I32 | RType::I64)
    }

    /// `rawptr` or `*T`.
    pub fn is_pointer(&self) -> bool {
        matches!(self, RType::RawPtr | RType::Pointer(_))
    }

    pub fn is_func(&self) -> bool {
        matches!(self, RType::Func(_, _))
    }

    pub fn is_error(&self) -> bool {
        matches!(self, RType::Error)
    }

    /// Width in bits of an integer type (0 for anything else).
    pub fn int_width(&self) -> u32 {
        match self {
            RType::I8 | RType::U8 => 8,
            RType::I16 | RType::U16 => 16,
            RType::I32 | RType::U32 => 32,
            RType::I64 | RType::U64 => 64,
            _ => 0,
        }
    }

    /// Inclusive range of values representable by an integer type.
    pub fn int_range(&self) -> Option<(i128, i128)> {
        Some(match self {
            RType::I8 => (i8::MIN as i128, i8::MAX as i128),
            RType::I16 => (i16::MIN as i128, i16::MAX as i128),
            RType::I32 => (i32::MIN as i128, i32::MAX as i128),
            RType::I64 => (i64::MIN as i128, i64::MAX as i128),
            RType::U8 => (0, u8::MAX as i128),
            RType::U16 => (0, u16::MAX as i128),
            RType::U32 => (0, u32::MAX as i128),
            RType::U64 => (0, u64::MAX as i128),
            _ => return None,
        })
    }
}

/// Renders a type the way it is written in Marrow source code.
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
        RType::Pointer(t) => format!("*{}", type_name(t)),
        RType::Slice(t) => match t.as_ref() {
            // `*T[]` would parse as a pointer to a slice, so be explicit.
            RType::Pointer(_) => format!("({})[]", type_name(t)),
            _ => format!("{}[]", type_name(t)),
        },
        RType::Array(n, t) => format!("[{}]{}", n, type_name(t)),
        RType::Struct(n) => n.clone(),
        RType::Func(params, ret) => {
            let ps: Vec<String> = params.iter().map(type_name).collect();
            if **ret == RType::Void {
                format!("fn({})", ps.join(", "))
            } else {
                format!("fn({}) -> {}", ps.join(", "), type_name(ret))
            }
        }
        RType::Void => "void".to_string(),
        RType::Error => "{error}".to_string(),
    }
}

pub fn builtin_rtype(s: &str) -> Option<RType> {
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
