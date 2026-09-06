use crate::{
    common::{
        errors::{ErrorSeverity, SemanticCheckerError},
        span::Span,
        types::Type,
    },
    semantic::type_alu::TypeALU,
};

impl TypeALU {
    pub(in crate::semantic) fn greater(t1: Type, t2: Type, span: Span) -> Result<Type, SemanticCheckerError> {
        Self::check_comparison(t1, t2, "greater", span)
    }

    pub(in crate::semantic) fn greater_or_equal(t1: Type, t2: Type, span: Span) -> Result<Type, SemanticCheckerError> {
        Self::check_comparison(t1, t2, "greater or equal", span)
    }

    pub(in crate::semantic) fn less(t1: Type, t2: Type, span: Span) -> Result<Type, SemanticCheckerError> {
        Self::check_comparison(t1, t2, "less", span)
    }

    pub(in crate::semantic) fn less_or_equal(t1: Type, t2: Type, span: Span) -> Result<Type, SemanticCheckerError> {
        Self::check_comparison(t1, t2, "less or equal", span)
    }

    pub(in crate::semantic) fn equal(t1: Type, t2: Type, span: Span) -> Result<Type, SemanticCheckerError> {
        Self::check_equality(t1, t2, "equal", span)
    }

    pub(in crate::semantic) fn not_equal(t1: Type, t2: Type, span: Span) -> Result<Type, SemanticCheckerError> {
        Self::check_equality(t1, t2, "not equal", span)
    }

    fn check_equality(t1: Type, t2: Type, op_name: &str, span: Span) -> Result<Type, SemanticCheckerError> {
        match (t1, t2) {
            // Integer types
            (
                Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::Char,
                Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::Char,
            ) => Ok(Type::Bool),

            // Floating point
            (Type::F64, Type::F64) => Ok(Type::Bool),

            // Other equality-comparable types
            (Type::Str, Type::Str) => Ok(Type::Bool),
            (Type::Bool, Type::Bool) => Ok(Type::Bool),

            (a, b) => Err(SemanticCheckerError::new(
                ErrorSeverity::HIGH,
                format!("Cannot perform {} between values of type '{}' and '{}'.", op_name, a, b),
                span,
            )),
        }
    }

    fn check_comparison(t1: Type, t2: Type, op_name: &str, span: Span) -> Result<Type, SemanticCheckerError> {
        match (t1, t2) {
            // Integer types
            (
                Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::U8 | Type::U16 | Type::U32 | Type::U64,
                Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::U8 | Type::U16 | Type::U32 | Type::U64,
            ) => Ok(Type::Bool),

            // Floating point
            (Type::F64, Type::F64) => Ok(Type::Bool),

            (a, b) => Err(SemanticCheckerError::new(
                ErrorSeverity::HIGH,
                format!("Cannot perform {} between values of type '{}' and '{}'.", op_name, a, b),
                span,
            )),
        }
    }
}
