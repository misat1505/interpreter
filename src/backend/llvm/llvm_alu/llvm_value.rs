use inkwell::context::Context;
use inkwell::types::{BasicTypeEnum, IntType, StructType};
use inkwell::values::{BasicValueEnum, FloatValue, IntValue, PointerValue};
use inkwell::AddressSpace;

use crate::common::span::Span;
use crate::common::{
    errors::{CompilerError, ErrorSeverity, IError},
    types::Type,
};

#[derive(Debug, Clone)]
pub enum LlvmValue<'ctx> {
    I8(IntValue<'ctx>),
    I16(IntValue<'ctx>),
    I32(IntValue<'ctx>),
    I64(IntValue<'ctx>),

    U8(IntValue<'ctx>),
    U16(IntValue<'ctx>),
    U32(IntValue<'ctx>),
    U64(IntValue<'ctx>),

    F64(FloatValue<'ctx>),
    /// Pointer to a `StrHeader { refcount: i64, data: i8* }`. Never a raw
    /// `i8*` into character data directly - always go through
    /// `str_header_type` field accessors to reach the character buffer.
    Str(PointerValue<'ctx>),
    Char(IntValue<'ctx>),
    Bool(IntValue<'ctx>),

    /// Pointer to a `VecHeader { refcount: i64, data: T*, length: i64, capacity: i64 }`.
    Vector(PointerValue<'ctx>, Box<Type>),
    /// Pointer to the struct's fields, followed by a trailing `refcount: i64` field.
    Struct(PointerValue<'ctx>, Box<Type>),
}

// ============================================================================
// Field indices for the heap object headers used by the refcounting runtime.
// ============================================================================

/// Field index of the `refcount: i64` field inside a `StrHeader`.
pub const STR_REFCOUNT: u32 = 0;
/// Field index of the `data: i8*` field inside a `StrHeader`.
pub const STR_DATA: u32 = 1;

/// Field index of the `refcount: i64` field inside a `VecHeader`.
pub const VEC_REFCOUNT: u32 = 0;
/// Field index of the `data: T*` field inside a `VecHeader`.
pub const VEC_DATA: u32 = 1;
/// Field index of the `length: i64` field inside a `VecHeader`.
pub const VEC_LENGTH: u32 = 2;
/// Field index of the `capacity: i64` field inside a `VecHeader`.
pub const VEC_CAPACITY: u32 = 3;

impl<'ctx> LlvmValue<'ctx> {
    pub fn to_type(&self) -> Type {
        match self {
            LlvmValue::I8(_) => Type::I8,
            LlvmValue::I16(_) => Type::I16,
            LlvmValue::I32(_) => Type::I32,
            LlvmValue::I64(_) => Type::I64,

            LlvmValue::U8(_) => Type::U8,
            LlvmValue::U16(_) => Type::U16,
            LlvmValue::U32(_) => Type::U32,
            LlvmValue::U64(_) => Type::U64,

            LlvmValue::F64(_) => Type::F64,
            LlvmValue::Str(_) => Type::Str,
            LlvmValue::Char(_) => Type::Char,
            LlvmValue::Bool(_) => Type::Bool,

            LlvmValue::Vector(_, inner) => Type::Vector(inner.clone()),
            LlvmValue::Struct(_, ty) => (**ty).clone(),
        }
    }

    /// Whether this value is a heap object managed by the refcounting
    /// runtime (i.e. `retain_value`/`release_value` do something for it).
    pub fn is_refcounted(&self) -> bool {
        matches!(self, LlvmValue::Str(_) | LlvmValue::Vector(_, _) | LlvmValue::Struct(_, _))
    }

    pub fn as_basic_value_enum(&self) -> BasicValueEnum<'ctx> {
        match self {
            LlvmValue::I8(v) => (*v).into(),
            LlvmValue::I16(v) => (*v).into(),
            LlvmValue::I32(v) => (*v).into(),
            LlvmValue::I64(v) => (*v).into(),

            LlvmValue::U8(v) => (*v).into(),
            LlvmValue::U16(v) => (*v).into(),
            LlvmValue::U32(v) => (*v).into(),
            LlvmValue::U64(v) => (*v).into(),

            LlvmValue::F64(v) => (*v).into(),
            LlvmValue::Str(v) => (*v).into(),
            LlvmValue::Char(v) => (*v).into(),
            LlvmValue::Bool(v) => (*v).into(),

            LlvmValue::Vector(v, _) => (*v).into(),
            LlvmValue::Struct(v, _) => (*v).into(),
        }
    }

    pub fn into_int_value(self, span: Span) -> Result<IntValue<'ctx>, Box<dyn IError>> {
        match self {
            LlvmValue::Bool(v) => Ok(v),

            other => Err(Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                format!("Expected a boolean condition, got '{}'.", other.to_type()),
                span,
            ))),
        }
    }

    pub fn from_basic_value_enum(value: BasicValueEnum<'ctx>, target_type: &Type) -> Self {
        match (target_type, value) {
            (Type::I8, BasicValueEnum::IntValue(v)) => LlvmValue::I8(v),
            (Type::I16, BasicValueEnum::IntValue(v)) => LlvmValue::I16(v),
            (Type::I32, BasicValueEnum::IntValue(v)) => LlvmValue::I32(v),
            (Type::I64, BasicValueEnum::IntValue(v)) => LlvmValue::I64(v),

            (Type::U8, BasicValueEnum::IntValue(v)) => LlvmValue::U8(v),
            (Type::U16, BasicValueEnum::IntValue(v)) => LlvmValue::U16(v),
            (Type::U32, BasicValueEnum::IntValue(v)) => LlvmValue::U32(v),
            (Type::U64, BasicValueEnum::IntValue(v)) => LlvmValue::U64(v),

            (Type::F64, BasicValueEnum::FloatValue(v)) => LlvmValue::F64(v),

            (Type::Str, BasicValueEnum::PointerValue(v)) => LlvmValue::Str(v),
            (Type::Char, BasicValueEnum::IntValue(v)) => LlvmValue::Char(v),

            (Type::Bool, BasicValueEnum::IntValue(v)) => LlvmValue::Bool(v),

            (Type::Vector(inner), BasicValueEnum::PointerValue(v)) => LlvmValue::Vector(v, inner.clone()),
            (Type::Struct { .. }, BasicValueEnum::PointerValue(v)) => LlvmValue::Struct(v, Box::new(target_type.clone())),

            _ => unreachable!("BasicValueEnum variant should always match the declared Type"),
        }
    }

    pub fn type_to_basic_type_enum(ty: &Type, context: &'ctx Context) -> Option<BasicTypeEnum<'ctx>> {
        match ty {
            Type::I8 => Some(context.i8_type().into()),
            Type::I16 => Some(context.i16_type().into()),
            Type::I32 => Some(context.i32_type().into()),
            Type::I64 => Some(context.i64_type().into()),

            Type::U8 => Some(context.i8_type().into()),
            Type::U16 => Some(context.i16_type().into()),
            Type::U32 => Some(context.i32_type().into()),
            Type::U64 => Some(context.i64_type().into()),

            Type::F64 => Some(context.f64_type().into()),

            Type::Str => Some(context.ptr_type(AddressSpace::default()).into()),
            Type::Char => Some(context.i8_type().into()),

            Type::Bool => Some(context.bool_type().into()),

            Type::Vector(_) => Some(context.ptr_type(AddressSpace::default()).into()),
            Type::Struct { .. } => Some(context.ptr_type(AddressSpace::default()).into()),

            _ => None,
        }
    }

    /// `VecHeader { refcount: i64, data: T*, length: i64, capacity: i64 }`.
    ///
    /// Field order matches the `VEC_*` index constants above.
    pub fn vector_struct_type(context: &'ctx Context) -> StructType<'ctx> {
        let ptr_type = context.ptr_type(AddressSpace::default());
        let i64_type = context.i64_type();

        context.struct_type(
            &[
                i64_type.into(), // refcount
                ptr_type.into(), // data
                i64_type.into(), // length
                i64_type.into(), // capacity
            ],
            false,
        )
    }

    /// `StrHeader { refcount: i64, data: i8* }`.
    ///
    /// Field order matches the `STR_*` index constants above.
    pub fn str_header_type(context: &'ctx Context) -> StructType<'ctx> {
        let ptr_type = context.ptr_type(AddressSpace::default());
        let i64_type = context.i64_type();

        context.struct_type(
            &[
                i64_type.into(), // refcount
                ptr_type.into(), // data
            ],
            false,
        )
    }

    pub fn element_byte_size(inner_type: &Type, i64_type: IntType<'ctx>, span: Span) -> Result<IntValue<'ctx>, Box<dyn IError>> {
        let size: u64 = match inner_type {
            Type::I8 | Type::U8 => 1,

            Type::I16 | Type::U16 => 2,

            Type::I32 | Type::U32 => 4,

            Type::I64 | Type::U64 => 8,

            Type::F64 => 8,

            Type::Bool => 1,
            Type::Char => 1,

            Type::Str => 8,           // TODO: 64-bit platform only
            Type::Vector(_) => 8,     // TODO: 64-bit platform only
            Type::Struct { .. } => 8, // TODO: 64-bit platform only

            other => {
                return Err(Box::new(CompilerError::new(
                    ErrorSeverity::HIGH,
                    format!("Compiling vectors of type '{}' is not yet supported. 6", other),
                    span,
                )))
            }
        };

        Ok(i64_type.const_int(size, false))
    }

    pub fn into_i64_value(self, span: Span) -> Result<IntValue<'ctx>, Box<dyn IError>> {
        match self {
            LlvmValue::I64(v) => Ok(v),

            other => Err(Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                format!("Expected an i64 index, got '{}'.", other.to_type()),
                span,
            ))),
        }
    }

    pub fn into_str_value(self, span: Span) -> Result<PointerValue<'ctx>, Box<dyn IError>> {
        match self {
            LlvmValue::Str(v) => Ok(v),

            other => Err(Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                format!("Expected a string, got '{}'.", other.to_type()),
                span,
            ))),
        }
    }

    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            LlvmValue::I8(_)
                | LlvmValue::I16(_)
                | LlvmValue::I32(_)
                | LlvmValue::I64(_)
                | LlvmValue::U8(_)
                | LlvmValue::U16(_)
                | LlvmValue::U32(_)
                | LlvmValue::U64(_)
        )
    }

    pub fn into_char_value(self, span: Span) -> Result<IntValue<'ctx>, Box<dyn IError>> {
        match self {
            LlvmValue::Char(v) => Ok(v),

            other => Err(Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                format!("Expected a char, got '{}'.", other.to_type()),
                span,
            ))),
        }
    }
}
