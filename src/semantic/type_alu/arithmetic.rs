use crate::{
    common::{
        errors::{ErrorSeverity, SemanticCheckerError},
        span::Span,
        types::Type,
    },
    semantic::type_alu::TypeALU,
};

impl TypeALU {
    pub(in crate::semantic) fn check_numeric_operation(t1: Type, t2: Type, op_name: &str, span: Span) -> Result<Type, SemanticCheckerError> {
        match (t1, t2) {
            (Type::I8, Type::I8) => Ok(Type::I8),
            (Type::I16, Type::I16) => Ok(Type::I16),
            (Type::I32, Type::I32) => Ok(Type::I32),
            (Type::I64, Type::I64) => Ok(Type::I64),

            (Type::U8, Type::U8) => Ok(Type::U8),
            (Type::U16, Type::U16) => Ok(Type::U16),
            (Type::U32, Type::U32) => Ok(Type::U32),
            (Type::U64, Type::U64) => Ok(Type::U64),

            (Type::F64, Type::F64) => Ok(Type::F64),
            (a, b) => Err(SemanticCheckerError::new(
                ErrorSeverity::HIGH,
                format!("Cannot perform {} between values of type '{}' and '{}'.", op_name, a, b),
                span,
            )),
        }
    }

    pub(in crate::semantic) fn add(t1: Type, t2: Type, span: Span) -> Result<Type, SemanticCheckerError> {
        match (t1, t2) {
            (Type::I8, Type::I8) => Ok(Type::I8),
            (Type::I16, Type::I16) => Ok(Type::I16),
            (Type::I32, Type::I32) => Ok(Type::I32),
            (Type::I64, Type::I64) => Ok(Type::I64),

            (Type::U8, Type::U8) => Ok(Type::U8),
            (Type::U16, Type::U16) => Ok(Type::U16),
            (Type::U32, Type::U32) => Ok(Type::U32),
            (Type::U64, Type::U64) => Ok(Type::U64),

            (Type::F64, Type::F64) => Ok(Type::F64),
            (Type::Str, Type::Str) => Ok(Type::Str),
            (Type::Char, Type::Char) => Ok(Type::Str),
            (Type::Str, Type::Char) => Ok(Type::Str),
            (Type::Char, Type::Str) => Ok(Type::Str),
            (a, b) => Err(SemanticCheckerError::new(
                ErrorSeverity::HIGH,
                format!("Cannot perform addition between values of type '{}' and '{}'.", a, b),
                span,
            )),
        }
    }

    pub(in crate::semantic) fn subtract(t1: Type, t2: Type, span: Span) -> Result<Type, SemanticCheckerError> {
        Self::check_numeric_operation(t1, t2, "subtraction", span)
    }

    pub(in crate::semantic) fn multiplication(t1: Type, t2: Type, span: Span) -> Result<Type, SemanticCheckerError> {
        Self::check_numeric_operation(t1, t2, "multiplication", span)
    }

    pub(in crate::semantic) fn division(t1: Type, t2: Type, span: Span) -> Result<Type, SemanticCheckerError> {
        Self::check_numeric_operation(t1, t2, "division", span)
    }

    pub(in crate::semantic) fn modulo(t1: Type, t2: Type, span: Span) -> Result<Type, SemanticCheckerError> {
        match (t1, t2) {
            (Type::I8, Type::I8) => Ok(Type::I8),
            (Type::I16, Type::I16) => Ok(Type::I16),
            (Type::I32, Type::I32) => Ok(Type::I32),
            (Type::I64, Type::I64) => Ok(Type::I64),

            (Type::U8, Type::U8) => Ok(Type::U8),
            (Type::U16, Type::U16) => Ok(Type::U16),
            (Type::U32, Type::U32) => Ok(Type::U32),
            (Type::U64, Type::U64) => Ok(Type::U64),
            (a, b) => Err(SemanticCheckerError::new(
                ErrorSeverity::HIGH,
                format!("Cannot perform modulo between values of type '{}' and '{}'.", a, b),
                span,
            )),
        }
    }
}
