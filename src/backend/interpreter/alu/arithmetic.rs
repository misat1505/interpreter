use crate::backend::interpreter::alu::value::Value;
use crate::backend::interpreter::alu::ALU;
use crate::common::errors::{ComputationError, ErrorSeverity};
use crate::common::span::Span;

impl ALU {
    fn check_int_operation<F>(val1: &Value, val2: &Value, op: F, op_name: &str, span: Span) -> Result<Value, ComputationError>
    where
        F: Fn() -> Option<Value>,
    {
        match op() {
            Some(result) => Ok(result),

            None => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!(
                    "Overflow occurred when performing {} on {} and {}.",
                    op_name,
                    val1.to_type(),
                    val2.to_type()
                ),
                span,
            )),
        }
    }

    fn check_float_operation<F>(val1: &Value, val2: &Value, op: F, op_name: &str, span: Span) -> Result<Value, ComputationError>
    where
        F: Fn(f64, f64) -> f64,
    {
        match (val1, val2) {
            (Value::F64(a), Value::F64(b)) => {
                let result = op(*a, *b);

                if result.is_infinite() || result.is_nan() {
                    Err(ComputationError::new(
                        ErrorSeverity::HIGH,
                        format!("Invalid result when performing {} on f64s.", op_name),
                        span,
                    ))
                } else {
                    Ok(Value::F64(result))
                }
            }

            _ => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!(
                    "Cannot perform {} between values of type '{}' and '{}'.",
                    op_name,
                    val1.to_type(),
                    val2.to_type()
                ),
                span,
            )),
        }
    }

    pub(in crate::backend::interpreter) fn add(val1: Value, val2: Value, span: Span) -> Result<Value, ComputationError> {
        match (&val1, &val2) {
            (Value::I8(a), Value::I8(b)) => Self::check_int_operation(&val1, &val2, || a.checked_add(*b).map(Value::I8), "addition", span),
            (Value::I16(a), Value::I16(b)) => Self::check_int_operation(&val1, &val2, || a.checked_add(*b).map(Value::I16), "addition", span),
            (Value::I32(a), Value::I32(b)) => Self::check_int_operation(&val1, &val2, || a.checked_add(*b).map(Value::I32), "addition", span),
            (Value::I64(a), Value::I64(b)) => Self::check_int_operation(&val1, &val2, || a.checked_add(*b).map(Value::I64), "addition", span),
            (Value::U8(a), Value::U8(b)) => Self::check_int_operation(&val1, &val2, || a.checked_add(*b).map(Value::U8), "addition", span),
            (Value::U16(a), Value::U16(b)) => Self::check_int_operation(&val1, &val2, || a.checked_add(*b).map(Value::U16), "addition", span),
            (Value::U32(a), Value::U32(b)) => Self::check_int_operation(&val1, &val2, || a.checked_add(*b).map(Value::U32), "addition", span),
            (Value::U64(a), Value::U64(b)) => Self::check_int_operation(&val1, &val2, || a.checked_add(*b).map(Value::U64), "addition", span),
            (Value::F64(_), Value::F64(_)) => Self::check_float_operation(&val1, &val2, |a, b| a + b, "addition", span),
            (Value::String(a), Value::String(b)) => Ok(Value::String(a.clone() + b)),
            (Value::Char(a), Value::Char(b)) => Ok(Value::String(format!("{}{}", a, b))),
            (Value::Char(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
            (Value::String(a), Value::Char(b)) => Ok(Value::String(format!("{}{}", a, b))),
            (a, b) => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!("Cannot perform addition between values of type '{}' and '{}'.", a.to_type(), b.to_type()),
                span,
            )),
        }
    }

    pub(in crate::backend::interpreter) fn subtract(val1: Value, val2: Value, span: Span) -> Result<Value, ComputationError> {
        match (&val1, &val2) {
            (Value::I8(a), Value::I8(b)) => Self::check_int_operation(&val1, &val2, || a.checked_sub(*b).map(Value::I8), "subtraction", span),
            (Value::I16(a), Value::I16(b)) => Self::check_int_operation(&val1, &val2, || a.checked_sub(*b).map(Value::I16), "subtraction", span),
            (Value::I32(a), Value::I32(b)) => Self::check_int_operation(&val1, &val2, || a.checked_sub(*b).map(Value::I32), "subtraction", span),
            (Value::I64(a), Value::I64(b)) => Self::check_int_operation(&val1, &val2, || a.checked_sub(*b).map(Value::I64), "subtraction", span),
            (Value::U8(a), Value::U8(b)) => Self::check_int_operation(&val1, &val2, || a.checked_sub(*b).map(Value::U8), "subtraction", span),
            (Value::U16(a), Value::U16(b)) => Self::check_int_operation(&val1, &val2, || a.checked_sub(*b).map(Value::U16), "subtraction", span),
            (Value::U32(a), Value::U32(b)) => Self::check_int_operation(&val1, &val2, || a.checked_sub(*b).map(Value::U32), "subtraction", span),
            (Value::U64(a), Value::U64(b)) => Self::check_int_operation(&val1, &val2, || a.checked_sub(*b).map(Value::U64), "subtraction", span),
            (Value::F64(_), Value::F64(_)) => Self::check_float_operation(&val1, &val2, |a, b| a - b, "subtraction", span),
            (a, b) => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!(
                    "Cannot perform subtraction between values of type '{}' and '{}'.",
                    a.to_type(),
                    b.to_type()
                ),
                span,
            )),
        }
    }

    pub(in crate::backend::interpreter) fn multiplication(val1: Value, val2: Value, span: Span) -> Result<Value, ComputationError> {
        match (&val1, &val2) {
            (Value::I8(a), Value::I8(b)) => Self::check_int_operation(&val1, &val2, || a.checked_mul(*b).map(Value::I8), "multiplication", span),
            (Value::I16(a), Value::I16(b)) => Self::check_int_operation(&val1, &val2, || a.checked_mul(*b).map(Value::I16), "multiplication", span),
            (Value::I32(a), Value::I32(b)) => Self::check_int_operation(&val1, &val2, || a.checked_mul(*b).map(Value::I32), "multiplication", span),
            (Value::I64(a), Value::I64(b)) => Self::check_int_operation(&val1, &val2, || a.checked_mul(*b).map(Value::I64), "multiplication", span),
            (Value::U8(a), Value::U8(b)) => Self::check_int_operation(&val1, &val2, || a.checked_mul(*b).map(Value::U8), "multiplication", span),
            (Value::U16(a), Value::U16(b)) => Self::check_int_operation(&val1, &val2, || a.checked_mul(*b).map(Value::U16), "multiplication", span),
            (Value::U32(a), Value::U32(b)) => Self::check_int_operation(&val1, &val2, || a.checked_mul(*b).map(Value::U32), "multiplication", span),
            (Value::U64(a), Value::U64(b)) => Self::check_int_operation(&val1, &val2, || a.checked_mul(*b).map(Value::U64), "multiplication", span),
            (Value::F64(_), Value::F64(_)) => Self::check_float_operation(&val1, &val2, |a, b| a * b, "multiplication", span),
            (a, b) => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!(
                    "Cannot perform multiplication between values of type '{}' and '{}'.",
                    a.to_type(),
                    b.to_type()
                ),
                span,
            )),
        }
    }

    pub(in crate::backend::interpreter) fn division(val1: Value, val2: Value, span: Span) -> Result<Value, ComputationError> {
        match (&val1, &val2) {
            (Value::I8(a), Value::I8(b)) => Self::check_int_operation(&val1, &val2, || a.checked_div(*b).map(Value::I8), "division", span),
            (Value::I16(a), Value::I16(b)) => Self::check_int_operation(&val1, &val2, || a.checked_div(*b).map(Value::I16), "division", span),
            (Value::I32(a), Value::I32(b)) => Self::check_int_operation(&val1, &val2, || a.checked_div(*b).map(Value::I32), "division", span),
            (Value::I64(a), Value::I64(b)) => Self::check_int_operation(&val1, &val2, || a.checked_div(*b).map(Value::I64), "division", span),
            (Value::U8(a), Value::U8(b)) => Self::check_int_operation(&val1, &val2, || a.checked_div(*b).map(Value::U8), "division", span),
            (Value::U16(a), Value::U16(b)) => Self::check_int_operation(&val1, &val2, || a.checked_div(*b).map(Value::U16), "division", span),
            (Value::U32(a), Value::U32(b)) => Self::check_int_operation(&val1, &val2, || a.checked_div(*b).map(Value::U32), "division", span),
            (Value::U64(a), Value::U64(b)) => Self::check_int_operation(&val1, &val2, || a.checked_div(*b).map(Value::U64), "division", span),
            (Value::F64(_), Value::F64(_)) => Self::check_float_operation(&val1, &val2, |a, b| a / b, "division", span),
            (a, b) => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!("Cannot perform division between values of type '{}' and '{}'.", a.to_type(), b.to_type()),
                span,
            )),
        }
    }

    pub(in crate::backend::interpreter) fn modulo(val1: Value, val2: Value, span: Span) -> Result<Value, ComputationError> {
        match (&val1, &val2) {
            (Value::I8(a), Value::I8(b)) => Self::check_int_operation(&val1, &val2, || a.checked_rem(*b).map(Value::I8), "modulo", span),
            (Value::I16(a), Value::I16(b)) => Self::check_int_operation(&val1, &val2, || a.checked_rem(*b).map(Value::I16), "modulo", span),
            (Value::I32(a), Value::I32(b)) => Self::check_int_operation(&val1, &val2, || a.checked_rem(*b).map(Value::I32), "modulo", span),
            (Value::I64(a), Value::I64(b)) => Self::check_int_operation(&val1, &val2, || a.checked_rem(*b).map(Value::I64), "modulo", span),
            (Value::U8(a), Value::U8(b)) => Self::check_int_operation(&val1, &val2, || a.checked_rem(*b).map(Value::U8), "modulo", span),
            (Value::U16(a), Value::U16(b)) => Self::check_int_operation(&val1, &val2, || a.checked_rem(*b).map(Value::U16), "modulo", span),
            (Value::U32(a), Value::U32(b)) => Self::check_int_operation(&val1, &val2, || a.checked_rem(*b).map(Value::U32), "modulo", span),
            (Value::U64(a), Value::U64(b)) => Self::check_int_operation(&val1, &val2, || a.checked_rem(*b).map(Value::U64), "modulo", span),
            (a, b) => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!("Cannot perform modulo between values of type '{}' and '{}'.", a.to_type(), b.to_type()),
                span,
            )),
        }
    }
}
