use crate::{
    common::{
        errors::{ErrorSeverity, SemanticCheckerError},
        span::Span,
        types::Type,
    },
    semantic::type_alu::TypeALU,
};

impl TypeALU {
    pub(in crate::semantic) fn cast_to_type(from: Type, to_type: &Type, span: Span) -> Result<Type, SemanticCheckerError> {
        match (from, to_type) {
            // Integer -> Integer
            (
                Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::U8 | Type::U16 | Type::U32 | Type::U64,
                Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::U8 | Type::U16 | Type::U32 | Type::U64,
            ) => Ok(to_type.clone()),

            // Integer -> Str
            (Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::U8 | Type::U16 | Type::U32 | Type::U64, Type::Str) => Ok(Type::Str),

            // Integer -> F64
            (Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::U8 | Type::U16 | Type::U32 | Type::U64, Type::F64) => Ok(Type::F64),

            // Integer -> Bool
            (Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::U8 | Type::U16 | Type::U32 | Type::U64, Type::Bool) => Ok(Type::Bool),

            // F64 -> Integer
            (Type::F64, Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::U8 | Type::U16 | Type::U32 | Type::U64) => Ok(to_type.clone()),

            // Str -> Integer
            (Type::Str, Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::U8 | Type::U16 | Type::U32 | Type::U64) => Ok(to_type.clone()),

            // Bool -> Integer
            (Type::Bool, Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::U8 | Type::U16 | Type::U32 | Type::U64) => Ok(to_type.clone()),

            // F64 <-> Str
            (Type::F64, Type::Str) => Ok(Type::Str),
            (Type::Str, Type::F64) => Ok(Type::F64),

            // F64 <-> Bool
            (Type::F64, Type::Bool) => Ok(Type::Bool),
            (Type::Bool, Type::F64) => Ok(Type::F64),

            // Bool <-> Str
            (Type::Bool, Type::Str) => Ok(Type::Str),
            (Type::Str, Type::Bool) => Ok(Type::Bool),

            // Char <-> Str
            (Type::Char, Type::Str) => Ok(Type::Str),

            // Char <-> u8
            (Type::Char, Type::U8) => Ok(Type::U8),
            (Type::U8, Type::Char) => Ok(Type::Char),

            // Same type
            (value, target_type) if value == *target_type => Ok(value),

            // Everything else
            (value, target_type) => Err(SemanticCheckerError::new(
                ErrorSeverity::HIGH,
                format!("Cannot cast '{}' to '{}'.", value, target_type),
                span,
            )),
        }
    }
}
