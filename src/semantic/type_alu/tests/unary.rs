use crate::{
    common::{errors::IError, span::Span, types::Type},
    semantic::type_alu::TypeALU,
};

#[test]
fn boolean_negate_valid() {
    assert_eq!(TypeALU::boolean_negate(Type::Bool, Span::default()).unwrap(), Type::Bool);
}

#[test]
fn boolean_negate_invalid() {
    assert_eq!(
        TypeALU::boolean_negate(Type::I64, Span::default()).err().unwrap().message(),
        "Cannot perform boolean negation on type 'i64'."
    );
}

#[test]
fn arithmetic_negate_valid() {
    assert_eq!(TypeALU::arithmetic_negate(Type::I64, Span::default()).unwrap(), Type::I64);
    assert_eq!(TypeALU::arithmetic_negate(Type::F64, Span::default()).unwrap(), Type::F64);
}

#[test]
fn arithmetic_negate_invalid() {
    assert_eq!(
        TypeALU::arithmetic_negate(Type::Str, Span::default()).err().unwrap().message(),
        "Cannot perform arithmetic negation on type 'str'."
    );
}

#[test]
fn boolean_negate_rejects_all_non_bool_types() {
    for ty in [Type::I64, Type::F64, Type::Str, Type::Void] {
        assert!(TypeALU::boolean_negate(ty, Span::default()).is_err());
    }
}

#[test]
fn arithmetic_negate_rejects_all_non_numeric_types() {
    for ty in [Type::Bool, Type::Str, Type::Void] {
        assert!(TypeALU::arithmetic_negate(ty, Span::default()).is_err());
    }
}

#[test]
fn arithmetic_negate_all_signed_and_f64() {
    assert_eq!(TypeALU::arithmetic_negate(Type::I8, Span::default()).unwrap(), Type::I8);
    assert_eq!(TypeALU::arithmetic_negate(Type::I16, Span::default()).unwrap(), Type::I16);
    assert_eq!(TypeALU::arithmetic_negate(Type::I32, Span::default()).unwrap(), Type::I32);
    assert_eq!(TypeALU::arithmetic_negate(Type::I64, Span::default()).unwrap(), Type::I64);
    assert_eq!(TypeALU::arithmetic_negate(Type::F64, Span::default()).unwrap(), Type::F64);
}

#[test]
fn arithmetic_negate_rejects_unsigned_and_others() {
    let bad = [
        Type::U8,
        Type::U16,
        Type::U32,
        Type::U64,
        Type::Bool,
        Type::Str,
        Type::Char,
        Type::Void,
        Type::Vector(Box::new(Type::I64)),
    ];
    for t in bad {
        let r = TypeALU::arithmetic_negate(t.clone(), Span::default());
        assert!(r.is_err(), "expected error for {}", t);
        let msg = r.err().unwrap().message();
        assert!(msg.starts_with("Cannot perform arithmetic negation on type"), "msg: {}", msg);
    }
}

#[test]
fn boolean_negate_rejects_all_non_bool() {
    let bad = [
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
        Type::Void,
        Type::Vector(Box::new(Type::Bool)),
    ];
    for t in bad {
        let r = TypeALU::boolean_negate(t.clone(), Span::default());
        assert!(r.is_err(), "expected error for {}", t);
        let msg = r.err().unwrap().message();
        assert!(msg.starts_with("Cannot perform boolean negation on type"), "msg: {}", msg);
    }
}
