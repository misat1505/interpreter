use std::{collections::HashMap, fmt::Display};

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Bool,
    Str,
    Char,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F64,
    Void,
    Vector(Box<Type>),
    Struct {
        identifier: String,
        fields: HashMap<String, Type>,
    },
    // Internal, not available for the user. Placeholder created by the parser
    // when it encounters an identifier in type position (e.g. `Hobby[] hobbies`)
    // before it's known whether that name refers to a real declared type.
    // Must be resolved to a concrete `Type` (currently only `Struct`, later also
    // enum/class variants) by the resolver pass right after parsing; it should
    // never reach the type checker or later compilation stages.
    #[allow(dead_code)]
    Unresolved(String),
    #[allow(dead_code)]
    Any, // internal, not available for the user
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Bool => Ok(write!(f, "bool")?),
            Type::I8 => Ok(write!(f, "i8")?),
            Type::I16 => Ok(write!(f, "i16")?),
            Type::I32 => Ok(write!(f, "i32")?),
            Type::I64 => Ok(write!(f, "i64")?),
            Type::U8 => Ok(write!(f, "u8")?),
            Type::U16 => Ok(write!(f, "u16")?),
            Type::U32 => Ok(write!(f, "u32")?),
            Type::U64 => Ok(write!(f, "u64")?),
            Type::F64 => Ok(write!(f, "f64")?),
            Type::Str => Ok(write!(f, "str")?),
            Type::Char => Ok(write!(f, "char")?),
            Type::Void => Ok(write!(f, "void")?),
            Type::Vector(inner) => write!(f, "{}[]", inner),
            Type::Struct { identifier, .. } => write!(f, "{}", identifier),
            Type::Unresolved(name) => write!(f, "Unresolved({})", name),
            Type::Any => Ok(write!(f, "any")?),
        }
    }
}

impl Type {
    pub fn is_compatible(&self, other: &Type) -> bool {
        match (self, other) {
            (Type::Any, _) | (_, Type::Any) => true,
            (Type::Vector(a), Type::Vector(b)) => a.is_compatible(b),
            (Type::Struct { identifier: a, .. }, Type::Struct { identifier: b, .. }) => a == b,
            (a, b) => a == b,
        }
    }
}
