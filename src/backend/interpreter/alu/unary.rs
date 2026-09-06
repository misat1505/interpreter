use crate::backend::interpreter::alu::ALU;
use crate::{
    backend::interpreter::alu::value::Value,
    common::{
        errors::{ComputationError, ErrorSeverity},
        span::Span,
    },
};

impl ALU {
    pub(in crate::backend::interpreter) fn boolean_negate(val: Value, span: Span) -> Result<Value, ComputationError> {
        match val {
            Value::Bool(bool) => Ok(Value::Bool(!bool)),

            val => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!("Cannot perform boolean negation on type '{}'.", val.to_type()),
                span,
            )),
        }
    }

    pub(in crate::backend::interpreter) fn arithmetic_negate(val: Value, span: Span) -> Result<Value, ComputationError> {
        match val {
            Value::I8(value) => match value.checked_neg() {
                Some(result) => Ok(Value::I8(result)),
                None => Err(ComputationError::new(
                    ErrorSeverity::HIGH,
                    String::from("Overflow occurred when performing arithmetic negation on i8."),
                    span,
                )),
            },

            Value::I16(value) => match value.checked_neg() {
                Some(result) => Ok(Value::I16(result)),
                None => Err(ComputationError::new(
                    ErrorSeverity::HIGH,
                    String::from("Overflow occurred when performing arithmetic negation on i16."),
                    span,
                )),
            },

            Value::I32(value) => match value.checked_neg() {
                Some(result) => Ok(Value::I32(result)),
                None => Err(ComputationError::new(
                    ErrorSeverity::HIGH,
                    String::from("Overflow occurred when performing arithmetic negation on i32."),
                    span,
                )),
            },

            Value::I64(value) => match value.checked_neg() {
                Some(result) => Ok(Value::I64(result)),
                None => Err(ComputationError::new(
                    ErrorSeverity::HIGH,
                    String::from("Overflow occurred when performing arithmetic negation on i64."),
                    span,
                )),
            },

            Value::F64(value) => Ok(Value::F64(-value)),

            val => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!("Cannot perform arithmetic negation on type '{}'.", val.to_type()),
                span,
            )),
        }
    }
}
