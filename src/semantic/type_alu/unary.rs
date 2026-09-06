use crate::{
    common::{
        errors::{ErrorSeverity, SemanticCheckerError},
        span::Span,
        types::Type,
    },
    semantic::type_alu::TypeALU,
};

impl TypeALU {
    pub(in crate::semantic) fn boolean_negate(t: Type, span: Span) -> Result<Type, SemanticCheckerError> {
        match t {
            Type::Bool => Ok(Type::Bool),

            t => Err(SemanticCheckerError::new(
                ErrorSeverity::HIGH,
                format!("Cannot perform boolean negation on type '{}'.", t),
                span,
            )),
        }
    }

    pub(in crate::semantic) fn arithmetic_negate(t: Type, span: Span) -> Result<Type, SemanticCheckerError> {
        match t {
            Type::I8 => Ok(Type::I8),
            Type::I16 => Ok(Type::I16),
            Type::I32 => Ok(Type::I32),
            Type::I64 => Ok(Type::I64),
            Type::F64 => Ok(Type::F64),

            t => Err(SemanticCheckerError::new(
                ErrorSeverity::HIGH,
                format!("Cannot perform arithmetic negation on type '{}'.", t),
                span,
            )),
        }
    }
}
