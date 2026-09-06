use crate::backend::interpreter::alu::ALU;
use crate::{
    backend::interpreter::alu::value::Value,
    common::{
        errors::{ComputationError, ErrorSeverity},
        span::Span,
    },
};

impl ALU {
    pub(in crate::backend::interpreter) fn concatenation(val1: Value, val2: Value, span: Span) -> Result<Value, ComputationError> {
        match (val1, val2) {
            (Value::Bool(bool1), Value::Bool(bool2)) => Ok(Value::Bool(bool1 && bool2)),
            (a, b) => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!(
                    "Cannot perform concatenation between values of type '{}' and '{}'.",
                    a.to_type(),
                    b.to_type()
                ),
                span,
            )),
        }
    }

    pub(in crate::backend::interpreter) fn alternative(val1: Value, val2: Value, span: Span) -> Result<Value, ComputationError> {
        match (val1, val2) {
            (Value::Bool(bool1), Value::Bool(bool2)) => Ok(Value::Bool(bool1 || bool2)),
            (a, b) => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!(
                    "Cannot perform alternative between values of type '{}' and '{}'.",
                    a.to_type(),
                    b.to_type()
                ),
                span,
            )),
        }
    }
}
