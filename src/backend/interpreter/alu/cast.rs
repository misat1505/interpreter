use crate::backend::interpreter::alu::ALU;
use crate::{
    backend::interpreter::alu::value::Value,
    common::{
        errors::{ComputationError, ErrorSeverity},
        span::Span,
        types::Type,
    },
};

impl ALU {
    pub(in crate::backend::interpreter) fn cast_to_type(val: Value, to_type: &Type, span: Span) -> Result<Value, ComputationError> {
        match (val, to_type) {
            // Integer -> String
            (Value::I8(value), Type::Str) => Ok(Value::String(value.to_string())),
            (Value::I16(value), Type::Str) => Ok(Value::String(value.to_string())),
            (Value::I32(value), Type::Str) => Ok(Value::String(value.to_string())),
            (Value::I64(value), Type::Str) => Ok(Value::String(value.to_string())),

            (Value::U8(value), Type::Str) => Ok(Value::String(value.to_string())),
            (Value::U16(value), Type::Str) => Ok(Value::String(value.to_string())),
            (Value::U32(value), Type::Str) => Ok(Value::String(value.to_string())),
            (Value::U64(value), Type::Str) => Ok(Value::String(value.to_string())),

            // F64 -> String
            (Value::F64(value), Type::Str) => Ok(Value::String(value.to_string())),

            // Integer -> F64
            (Value::I8(value), Type::F64) => Ok(Value::F64(value as f64)),
            (Value::I16(value), Type::F64) => Ok(Value::F64(value as f64)),
            (Value::I32(value), Type::F64) => Ok(Value::F64(value as f64)),
            (Value::I64(value), Type::F64) => Ok(Value::F64(value as f64)),

            (Value::U8(value), Type::F64) => Ok(Value::F64(value as f64)),
            (Value::U16(value), Type::F64) => Ok(Value::F64(value as f64)),
            (Value::U32(value), Type::F64) => Ok(Value::F64(value as f64)),
            (Value::U64(value), Type::F64) => Ok(Value::F64(value as f64)),

            // F64 -> Integer
            (Value::F64(value), Type::I8) => {
                if value.is_finite() && value >= i8::MIN as f64 && value <= i8::MAX as f64 {
                    Ok(Value::I8(value as i8))
                } else {
                    Err(Self::invalid_cast(value, Type::I8, span))
                }
            }

            (Value::F64(value), Type::I16) => {
                if value.is_finite() && value >= i16::MIN as f64 && value <= i16::MAX as f64 {
                    Ok(Value::I16(value as i16))
                } else {
                    Err(Self::invalid_cast(value, Type::I16, span))
                }
            }

            (Value::F64(value), Type::I32) => {
                if value.is_finite() && value >= i32::MIN as f64 && value <= i32::MAX as f64 {
                    Ok(Value::I32(value as i32))
                } else {
                    Err(Self::invalid_cast(value, Type::I32, span))
                }
            }

            (Value::F64(value), Type::I64) => {
                if value.is_finite() && value >= i64::MIN as f64 && value <= i64::MAX as f64 {
                    Ok(Value::I64(value as i64))
                } else {
                    Err(Self::invalid_cast(value, Type::I64, span))
                }
            }

            (Value::F64(value), Type::U8) => {
                if value.is_finite() && value >= 0.0 && value <= u8::MAX as f64 {
                    Ok(Value::U8(value as u8))
                } else {
                    Err(Self::invalid_cast(value, Type::U8, span))
                }
            }

            (Value::F64(value), Type::U16) => {
                if value.is_finite() && value >= 0.0 && value <= u16::MAX as f64 {
                    Ok(Value::U16(value as u16))
                } else {
                    Err(Self::invalid_cast(value, Type::U16, span))
                }
            }

            (Value::F64(value), Type::U32) => {
                if value.is_finite() && value >= 0.0 && value <= u32::MAX as f64 {
                    Ok(Value::U32(value as u32))
                } else {
                    Err(Self::invalid_cast(value, Type::U32, span))
                }
            }

            (Value::F64(value), Type::U64) => {
                if value.is_finite() && value >= 0.0 && value <= u64::MAX as f64 {
                    Ok(Value::U64(value as u64))
                } else {
                    Err(Self::invalid_cast(value, Type::U64, span))
                }
            }

            // Integer -> Bool
            (Value::I8(value), Type::Bool) => Ok(Value::Bool(value > 0)),
            (Value::I16(value), Type::Bool) => Ok(Value::Bool(value > 0)),
            (Value::I32(value), Type::Bool) => Ok(Value::Bool(value > 0)),
            (Value::I64(value), Type::Bool) => Ok(Value::Bool(value > 0)),

            (Value::U8(value), Type::Bool) => Ok(Value::Bool(value > 0)),
            (Value::U16(value), Type::Bool) => Ok(Value::Bool(value > 0)),
            (Value::U32(value), Type::Bool) => Ok(Value::Bool(value > 0)),
            (Value::U64(value), Type::Bool) => Ok(Value::Bool(value > 0)),

            // F64 -> Bool
            (Value::F64(value), Type::Bool) => Ok(Value::Bool(value > 0.0)),

            // String -> Integer
            (Value::String(string), Type::I8) => match string.parse::<i8>() {
                Ok(value) => Ok(Value::I8(value)),
                Err(_) => Err(Self::invalid_string_cast(&string, "i8", span)),
            },

            (Value::String(string), Type::I16) => match string.parse::<i16>() {
                Ok(value) => Ok(Value::I16(value)),
                Err(_) => Err(Self::invalid_string_cast(&string, "i16", span)),
            },

            (Value::String(string), Type::I32) => match string.parse::<i32>() {
                Ok(value) => Ok(Value::I32(value)),
                Err(_) => Err(Self::invalid_string_cast(&string, "i32", span)),
            },

            (Value::String(string), Type::I64) => match string.parse::<i64>() {
                Ok(value) => Ok(Value::I64(value)),
                Err(_) => Err(Self::invalid_string_cast(&string, "i64", span)),
            },

            (Value::String(string), Type::U8) => match string.parse::<u8>() {
                Ok(value) => Ok(Value::U8(value)),
                Err(_) => Err(Self::invalid_string_cast(&string, "u8", span)),
            },

            (Value::String(string), Type::U16) => match string.parse::<u16>() {
                Ok(value) => Ok(Value::U16(value)),
                Err(_) => Err(Self::invalid_string_cast(&string, "u16", span)),
            },

            (Value::String(string), Type::U32) => match string.parse::<u32>() {
                Ok(value) => Ok(Value::U32(value)),
                Err(_) => Err(Self::invalid_string_cast(&string, "u32", span)),
            },

            (Value::String(string), Type::U64) => match string.parse::<u64>() {
                Ok(value) => Ok(Value::U64(value)),
                Err(_) => Err(Self::invalid_string_cast(&string, "u64", span)),
            },

            // String -> F64
            (Value::String(string), Type::F64) => match string.parse::<f64>() {
                Ok(value) => Ok(Value::F64(value)),
                Err(_) => Err(Self::invalid_string_cast(&string, "f64", span)),
            },

            // String -> Bool
            (Value::String(string), Type::Bool) => Ok(Value::Bool(!string.is_empty())),

            // Bool -> String
            (Value::Bool(value), Type::Str) => Ok(Value::String(if value { "true".to_owned() } else { "false".to_owned() })),

            // Bool -> Integer
            (Value::Bool(value), Type::I8) => Ok(Value::I8(if value { 1 } else { 0 })),
            (Value::Bool(value), Type::I16) => Ok(Value::I16(if value { 1 } else { 0 })),
            (Value::Bool(value), Type::I32) => Ok(Value::I32(if value { 1 } else { 0 })),
            (Value::Bool(value), Type::I64) => Ok(Value::I64(if value { 1 } else { 0 })),

            (Value::Bool(value), Type::U8) => Ok(Value::U8(if value { 1 } else { 0 })),
            (Value::Bool(value), Type::U16) => Ok(Value::U16(if value { 1 } else { 0 })),
            (Value::Bool(value), Type::U32) => Ok(Value::U32(if value { 1 } else { 0 })),
            (Value::Bool(value), Type::U64) => Ok(Value::U64(if value { 1 } else { 0 })),

            // Bool -> F64
            (Value::Bool(value), Type::F64) => Ok(Value::F64(if value { 1.0 } else { 0.0 })),

            // Char <-> u8
            (Value::Char(c), Type::U8) => Ok(Value::U8(c as u8)),
            (Value::U8(val), Type::Char) => Ok(Value::Char(val as char)),

            // Integer -> Integer
            (Value::I8(value), target_type) => Self::cast_signed_integer(value as i128, target_type, span),
            (Value::I16(value), target_type) => Self::cast_signed_integer(value as i128, target_type, span),
            (Value::I32(value), target_type) => Self::cast_signed_integer(value as i128, target_type, span),
            (Value::I64(value), target_type) => Self::cast_signed_integer(value as i128, target_type, span),
            (Value::U8(value), target_type) => Self::cast_unsigned_integer(value as u128, target_type, span),
            (Value::U16(value), target_type) => Self::cast_unsigned_integer(value as u128, target_type, span),
            (Value::U32(value), target_type) => Self::cast_unsigned_integer(value as u128, target_type, span),
            (Value::U64(value), target_type) => Self::cast_unsigned_integer(value as u128, target_type, span),

            // Char -> Str
            (Value::Char(c), Type::Str) => Ok(Value::String(String::from(c))),

            // Same type
            (value, target_type) if value.to_type() == *target_type => Ok(value),

            // Unsupported cast
            (value, target_type) => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!("Cannot cast '{}' to '{}'.", value, target_type),
                span,
            )),
        }
    }

    fn cast_signed_integer(value: i128, target_type: &Type, span: Span) -> Result<Value, ComputationError> {
        match target_type {
            Type::I8 => i8::try_from(value).map(Value::I8).map_err(|_| Self::invalid_cast(value, Type::I8, span)),

            Type::I16 => i16::try_from(value)
                .map(Value::I16)
                .map_err(|_| Self::invalid_cast(value, Type::I16, span)),

            Type::I32 => i32::try_from(value)
                .map(Value::I32)
                .map_err(|_| Self::invalid_cast(value, Type::I32, span)),

            Type::I64 => i64::try_from(value)
                .map(Value::I64)
                .map_err(|_| Self::invalid_cast(value, Type::I64, span)),

            Type::U8 => u8::try_from(value).map(Value::U8).map_err(|_| Self::invalid_cast(value, Type::U8, span)),

            Type::U16 => u16::try_from(value)
                .map(Value::U16)
                .map_err(|_| Self::invalid_cast(value, Type::U16, span)),

            Type::U32 => u32::try_from(value)
                .map(Value::U32)
                .map_err(|_| Self::invalid_cast(value, Type::U32, span)),

            Type::U64 => u64::try_from(value)
                .map(Value::U64)
                .map_err(|_| Self::invalid_cast(value, Type::U64, span)),

            _ => Err(Self::invalid_cast(value, target_type.clone(), span)),
        }
    }

    fn cast_unsigned_integer(value: u128, target_type: &Type, span: Span) -> Result<Value, ComputationError> {
        match target_type {
            Type::I8 => i8::try_from(value).map(Value::I8).map_err(|_| Self::invalid_cast(value, Type::I8, span)),

            Type::I16 => i16::try_from(value)
                .map(Value::I16)
                .map_err(|_| Self::invalid_cast(value, Type::I16, span)),

            Type::I32 => i32::try_from(value)
                .map(Value::I32)
                .map_err(|_| Self::invalid_cast(value, Type::I32, span)),

            Type::I64 => i64::try_from(value)
                .map(Value::I64)
                .map_err(|_| Self::invalid_cast(value, Type::I64, span)),

            Type::U8 => u8::try_from(value).map(Value::U8).map_err(|_| Self::invalid_cast(value, Type::U8, span)),

            Type::U16 => u16::try_from(value)
                .map(Value::U16)
                .map_err(|_| Self::invalid_cast(value, Type::U16, span)),

            Type::U32 => u32::try_from(value)
                .map(Value::U32)
                .map_err(|_| Self::invalid_cast(value, Type::U32, span)),

            Type::U64 => u64::try_from(value)
                .map(Value::U64)
                .map_err(|_| Self::invalid_cast(value, Type::U64, span)),

            _ => Err(Self::invalid_cast(value, target_type.clone(), span)),
        }
    }

    fn invalid_cast<T: std::fmt::Display>(value: T, target_type: Type, span: Span) -> ComputationError {
        ComputationError::new(ErrorSeverity::HIGH, format!("Cannot cast '{}' to '{}'.", value, target_type), span)
    }

    fn invalid_string_cast(value: &str, target_type: &str, span: Span) -> ComputationError {
        ComputationError::new(ErrorSeverity::HIGH, format!("Cannot cast String '{}' to '{}'.", value, target_type), span)
    }
}
