use crate::backend::interpreter::alu::value::Value;
use crate::backend::interpreter::alu::ALU;
use crate::common::errors::{ComputationError, ErrorSeverity};
use crate::common::span::Span;

impl ALU {
    fn comparison_error(val1: &Value, val2: &Value, op_name: &str, span: Span) -> ComputationError {
        ComputationError::new(
            ErrorSeverity::HIGH,
            format!(
                "Cannot perform {} between values of type '{}' and '{}'.",
                op_name,
                val1.to_type(),
                val2.to_type()
            ),
            span,
        )
    }

    pub(in crate::backend::interpreter) fn greater(val1: Value, val2: Value, span: Span) -> Result<Value, ComputationError> {
        match (&val1, &val2) {
            (Value::I8(a), Value::I8(b)) => Ok(Value::Bool(a > b)),
            (Value::I16(a), Value::I16(b)) => Ok(Value::Bool(a > b)),
            (Value::I32(a), Value::I32(b)) => Ok(Value::Bool(a > b)),
            (Value::I64(a), Value::I64(b)) => Ok(Value::Bool(a > b)),

            (Value::U8(a), Value::U8(b)) => Ok(Value::Bool(a > b)),
            (Value::U16(a), Value::U16(b)) => Ok(Value::Bool(a > b)),
            (Value::U32(a), Value::U32(b)) => Ok(Value::Bool(a > b)),
            (Value::U64(a), Value::U64(b)) => Ok(Value::Bool(a > b)),

            (Value::F64(a), Value::F64(b)) => Ok(Value::Bool(a > b)),

            (a, b) => Err(Self::comparison_error(a, b, "greater", span)),
        }
    }

    pub(in crate::backend::interpreter) fn greater_or_equal(val1: Value, val2: Value, span: Span) -> Result<Value, ComputationError> {
        match (&val1, &val2) {
            (Value::I8(a), Value::I8(b)) => Ok(Value::Bool(a >= b)),
            (Value::I16(a), Value::I16(b)) => Ok(Value::Bool(a >= b)),
            (Value::I32(a), Value::I32(b)) => Ok(Value::Bool(a >= b)),
            (Value::I64(a), Value::I64(b)) => Ok(Value::Bool(a >= b)),

            (Value::U8(a), Value::U8(b)) => Ok(Value::Bool(a >= b)),
            (Value::U16(a), Value::U16(b)) => Ok(Value::Bool(a >= b)),
            (Value::U32(a), Value::U32(b)) => Ok(Value::Bool(a >= b)),
            (Value::U64(a), Value::U64(b)) => Ok(Value::Bool(a >= b)),

            (Value::F64(a), Value::F64(b)) => Ok(Value::Bool(a >= b)),

            (a, b) => Err(Self::comparison_error(a, b, "greater or equal", span)),
        }
    }

    pub(in crate::backend::interpreter) fn less(val1: Value, val2: Value, span: Span) -> Result<Value, ComputationError> {
        match (&val1, &val2) {
            (Value::I8(a), Value::I8(b)) => Ok(Value::Bool(a < b)),
            (Value::I16(a), Value::I16(b)) => Ok(Value::Bool(a < b)),
            (Value::I32(a), Value::I32(b)) => Ok(Value::Bool(a < b)),
            (Value::I64(a), Value::I64(b)) => Ok(Value::Bool(a < b)),

            (Value::U8(a), Value::U8(b)) => Ok(Value::Bool(a < b)),
            (Value::U16(a), Value::U16(b)) => Ok(Value::Bool(a < b)),
            (Value::U32(a), Value::U32(b)) => Ok(Value::Bool(a < b)),
            (Value::U64(a), Value::U64(b)) => Ok(Value::Bool(a < b)),

            (Value::F64(a), Value::F64(b)) => Ok(Value::Bool(a < b)),

            (a, b) => Err(Self::comparison_error(a, b, "less", span)),
        }
    }

    pub(in crate::backend::interpreter) fn less_or_equal(val1: Value, val2: Value, span: Span) -> Result<Value, ComputationError> {
        match (&val1, &val2) {
            (Value::I8(a), Value::I8(b)) => Ok(Value::Bool(a <= b)),
            (Value::I16(a), Value::I16(b)) => Ok(Value::Bool(a <= b)),
            (Value::I32(a), Value::I32(b)) => Ok(Value::Bool(a <= b)),
            (Value::I64(a), Value::I64(b)) => Ok(Value::Bool(a <= b)),

            (Value::U8(a), Value::U8(b)) => Ok(Value::Bool(a <= b)),
            (Value::U16(a), Value::U16(b)) => Ok(Value::Bool(a <= b)),
            (Value::U32(a), Value::U32(b)) => Ok(Value::Bool(a <= b)),
            (Value::U64(a), Value::U64(b)) => Ok(Value::Bool(a <= b)),

            (Value::F64(a), Value::F64(b)) => Ok(Value::Bool(a <= b)),

            (a, b) => Err(Self::comparison_error(a, b, "less or equal", span)),
        }
    }

    pub(in crate::backend::interpreter) fn equal(val1: Value, val2: Value, span: Span) -> Result<Value, ComputationError> {
        match (&val1, &val2) {
            (Value::I8(a), Value::I8(b)) => Ok(Value::Bool(a == b)),
            (Value::I16(a), Value::I16(b)) => Ok(Value::Bool(a == b)),
            (Value::I32(a), Value::I32(b)) => Ok(Value::Bool(a == b)),
            (Value::I64(a), Value::I64(b)) => Ok(Value::Bool(a == b)),

            (Value::U8(a), Value::U8(b)) => Ok(Value::Bool(a == b)),
            (Value::U16(a), Value::U16(b)) => Ok(Value::Bool(a == b)),
            (Value::U32(a), Value::U32(b)) => Ok(Value::Bool(a == b)),
            (Value::U64(a), Value::U64(b)) => Ok(Value::Bool(a == b)),

            (Value::F64(a), Value::F64(b)) => Ok(Value::Bool(a == b)),
            (Value::String(a), Value::String(b)) => Ok(Value::Bool(a == b)),
            (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a == b)),
            (Value::Char(a), Value::Char(b)) => Ok(Value::Bool(a == b)),

            (a, b) => Err(Self::comparison_error(a, b, "equal", span)),
        }
    }

    pub(in crate::backend::interpreter) fn not_equal(val1: Value, val2: Value, span: Span) -> Result<Value, ComputationError> {
        match (&val1, &val2) {
            (Value::I8(a), Value::I8(b)) => Ok(Value::Bool(a != b)),
            (Value::I16(a), Value::I16(b)) => Ok(Value::Bool(a != b)),
            (Value::I32(a), Value::I32(b)) => Ok(Value::Bool(a != b)),
            (Value::I64(a), Value::I64(b)) => Ok(Value::Bool(a != b)),

            (Value::U8(a), Value::U8(b)) => Ok(Value::Bool(a != b)),
            (Value::U16(a), Value::U16(b)) => Ok(Value::Bool(a != b)),
            (Value::U32(a), Value::U32(b)) => Ok(Value::Bool(a != b)),
            (Value::U64(a), Value::U64(b)) => Ok(Value::Bool(a != b)),

            (Value::F64(a), Value::F64(b)) => Ok(Value::Bool(a != b)),
            (Value::String(a), Value::String(b)) => Ok(Value::Bool(a != b)),
            (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a != b)),
            (Value::Char(a), Value::Char(b)) => Ok(Value::Bool(a != b)),

            (a, b) => Err(Self::comparison_error(a, b, "not equal", span)),
        }
    }
}
