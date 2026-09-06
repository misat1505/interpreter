use inkwell::{context::Context, values::BasicValueEnum, AddressSpace};

use crate::{
    backend::llvm::llvm_alu::llvm_value::LlvmValue,
    common::{span::Span, types::Type},
};

#[test]
fn to_type_all_variants() {
    let context = Context::create();

    assert_eq!(LlvmValue::I8(context.i8_type().const_zero()).to_type(), Type::I8);
    assert_eq!(LlvmValue::I16(context.i16_type().const_zero()).to_type(), Type::I16);
    assert_eq!(LlvmValue::I32(context.i32_type().const_zero()).to_type(), Type::I32);
    assert_eq!(LlvmValue::I64(context.i64_type().const_zero()).to_type(), Type::I64);
    assert_eq!(LlvmValue::U8(context.i8_type().const_zero()).to_type(), Type::U8);
    assert_eq!(LlvmValue::U16(context.i16_type().const_zero()).to_type(), Type::U16);
    assert_eq!(LlvmValue::U32(context.i32_type().const_zero()).to_type(), Type::U32);
    assert_eq!(LlvmValue::U64(context.i64_type().const_zero()).to_type(), Type::U64);
    assert_eq!(LlvmValue::F64(context.f64_type().const_zero()).to_type(), Type::F64);
    assert_eq!(
        LlvmValue::Str(context.ptr_type(AddressSpace::default()).const_null()).to_type(),
        Type::Str
    );
    assert_eq!(LlvmValue::Char(context.i8_type().const_zero()).to_type(), Type::Char);
    assert_eq!(LlvmValue::Bool(context.bool_type().const_zero()).to_type(), Type::Bool);
    assert_eq!(
        LlvmValue::Vector(context.ptr_type(AddressSpace::default()).const_null(), Box::new(Type::I64)).to_type(),
        Type::Vector(Box::new(Type::I64))
    );
}

#[test]
fn as_basic_value_enum_roundtrip_kinds() {
    let context = Context::create();

    assert!(LlvmValue::I64(context.i64_type().const_int(1, true)).as_basic_value_enum().is_int_value());
    assert!(LlvmValue::F64(context.f64_type().const_float(1.0)).as_basic_value_enum().is_float_value());
    assert!(LlvmValue::Str(context.ptr_type(AddressSpace::default()).const_null())
        .as_basic_value_enum()
        .is_pointer_value());
    assert!(LlvmValue::Bool(context.bool_type().const_int(1, false))
        .as_basic_value_enum()
        .is_int_value());
    assert!(
        LlvmValue::Vector(context.ptr_type(AddressSpace::default()).const_null(), Box::new(Type::I8))
            .as_basic_value_enum()
            .is_pointer_value()
    );
}

#[test]
fn into_int_value_only_bool() {
    let context = Context::create();
    let span = Span::default();

    let b = LlvmValue::Bool(context.bool_type().const_int(1, false));
    assert!(b.into_int_value(span).is_ok());

    let i = LlvmValue::I64(context.i64_type().const_int(1, true));
    let err = i.into_int_value(span).unwrap_err();
    assert!(err.message().contains("Expected a boolean"));
}

#[test]
fn into_i64_value_only_i64() {
    let context = Context::create();
    let span = Span::default();

    let v = LlvmValue::I64(context.i64_type().const_int(42, true));
    assert!(v.into_i64_value(span).is_ok());

    let v = LlvmValue::I32(context.i32_type().const_int(42, true));
    let err = v.into_i64_value(span).unwrap_err();
    assert!(err.message().contains("Expected an i64"));
}

#[test]
fn into_str_value_only_str() {
    let context = Context::create();
    let span = Span::default();

    let v = LlvmValue::Str(context.ptr_type(AddressSpace::default()).const_null());
    assert!(v.into_str_value(span).is_ok());

    let v = LlvmValue::I64(context.i64_type().const_zero());
    let err = v.into_str_value(span).unwrap_err();
    assert!(err.message().contains("Expected a string"));
}

#[test]
fn into_char_value_only_char() {
    let context = Context::create();
    let span = Span::default();

    let v = LlvmValue::Char(context.i8_type().const_int(b'a' as u64, false));
    assert!(v.into_char_value(span).is_ok());

    let v = LlvmValue::U8(context.i8_type().const_int(65, false));
    let err = v.into_char_value(span).unwrap_err();
    assert!(err.message().contains("Expected a char"));
}

#[test]
fn is_integer() {
    let context = Context::create();

    assert!(LlvmValue::I8(context.i8_type().const_zero()).is_integer());
    assert!(LlvmValue::I16(context.i16_type().const_zero()).is_integer());
    assert!(LlvmValue::I32(context.i32_type().const_zero()).is_integer());
    assert!(LlvmValue::I64(context.i64_type().const_zero()).is_integer());
    assert!(LlvmValue::U8(context.i8_type().const_zero()).is_integer());
    assert!(LlvmValue::U16(context.i16_type().const_zero()).is_integer());
    assert!(LlvmValue::U32(context.i32_type().const_zero()).is_integer());
    assert!(LlvmValue::U64(context.i64_type().const_zero()).is_integer());

    assert!(!LlvmValue::F64(context.f64_type().const_zero()).is_integer());
    assert!(!LlvmValue::Bool(context.bool_type().const_zero()).is_integer());
    assert!(!LlvmValue::Char(context.i8_type().const_zero()).is_integer());
    assert!(!LlvmValue::Str(context.ptr_type(AddressSpace::default()).const_null()).is_integer());
}

#[test]
fn from_basic_value_enum_roundtrip() {
    let context = Context::create();

    let cases: Vec<(Type, BasicValueEnum)> = vec![
        (Type::I8, context.i8_type().const_int(1, true).into()),
        (Type::I16, context.i16_type().const_int(1, true).into()),
        (Type::I32, context.i32_type().const_int(1, true).into()),
        (Type::I64, context.i64_type().const_int(1, true).into()),
        (Type::U8, context.i8_type().const_int(1, false).into()),
        (Type::U16, context.i16_type().const_int(1, false).into()),
        (Type::U32, context.i32_type().const_int(1, false).into()),
        (Type::U64, context.i64_type().const_int(1, false).into()),
        (Type::F64, context.f64_type().const_float(1.0).into()),
        (Type::Str, context.ptr_type(AddressSpace::default()).const_null().into()),
        (Type::Char, context.i8_type().const_int(b'x' as u64, false).into()),
        (Type::Bool, context.bool_type().const_int(1, false).into()),
        (
            Type::Vector(Box::new(Type::I64)),
            context.ptr_type(AddressSpace::default()).const_null().into(),
        ),
    ];

    for (ty, basic) in cases {
        let llvm_val = LlvmValue::from_basic_value_enum(basic, &ty);
        assert_eq!(llvm_val.to_type(), ty);
    }
}

#[test]
fn type_to_basic_type_enum_all_supported() {
    let context = Context::create();

    let supported = [
        Type::I8,
        Type::I16,
        Type::I32,
        Type::I64,
        Type::U8,
        Type::U16,
        Type::U32,
        Type::U64,
        Type::F64,
        Type::Str,
        Type::Char,
        Type::Bool,
        Type::Vector(Box::new(Type::I64)),
    ];
    for ty in supported {
        assert!(LlvmValue::type_to_basic_type_enum(&ty, &context).is_some(), "expected Some for {}", ty);
    }

    assert!(LlvmValue::type_to_basic_type_enum(&Type::Void, &context).is_none());
}

#[test]
fn vector_struct_type_has_three_fields() {
    let context = Context::create();
    let st = LlvmValue::vector_struct_type(&context);
    // { i64, ptr, i64, i64 } — ref_count, data, len, capacity
    assert_eq!(st.count_fields(), 4);
}

#[test]
fn element_byte_size_known_types() {
    let context = Context::create();
    let i64_ty = context.i64_type();
    let span = Span::default();

    let cases = [
        (Type::I8, 1),
        (Type::U8, 1),
        (Type::I16, 2),
        (Type::U16, 2),
        (Type::I32, 4),
        (Type::U32, 4),
        (Type::I64, 8),
        (Type::U64, 8),
        (Type::F64, 8),
        (Type::Bool, 1),
        (Type::Char, 1),
        (Type::Str, 8),
        (Type::Vector(Box::new(Type::I64)), 8),
    ];

    for (ty, expected) in cases {
        let size = LlvmValue::element_byte_size(&ty, i64_ty, span).unwrap();
        assert_eq!(size.get_zero_extended_constant(), Some(expected));
    }
}

#[test]
fn element_byte_size_unsupported_fails() {
    let context = Context::create();
    let i64_ty = context.i64_type();
    let span = Span::default();

    let err = LlvmValue::element_byte_size(&Type::Void, i64_ty, span).unwrap_err();
    assert!(err.message().contains("not yet supported"));
}
