use crate::{
    common::{
        errors::{ErrorSeverity, SemanticCheckerError},
        span::Span,
        types::Type,
    },
    semantic::type_alu::TypeALU,
};

impl TypeALU {
    pub(in crate::semantic) fn concatenation(t1: Type, t2: Type, span: Span) -> Result<Type, SemanticCheckerError> {
        match (t1, t2) {
            (Type::Bool, Type::Bool) => Ok(Type::Bool),

            (a, b) => Err(SemanticCheckerError::new(
                ErrorSeverity::HIGH,
                format!("Cannot perform concatenation between values of type '{}' and '{}'.", a, b),
                span,
            )),
        }
    }

    pub(in crate::semantic) fn alternative(t1: Type, t2: Type, span: Span) -> Result<Type, SemanticCheckerError> {
        match (t1, t2) {
            (Type::Bool, Type::Bool) => Ok(Type::Bool),

            (a, b) => Err(SemanticCheckerError::new(
                ErrorSeverity::HIGH,
                format!("Cannot perform alternative between values of type '{}' and '{}'.", a, b),
                span,
            )),
        }
    }
}
