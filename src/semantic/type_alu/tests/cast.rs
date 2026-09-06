use crate::{
    common::{errors::IError, span::Span, types::Type},
    semantic::type_alu::TypeALU,
};

#[test]
fn cast_to_type_valid() {
    assert_eq!(TypeALU::cast_to_type(Type::I64, &Type::Str, Span::default()).unwrap(), Type::Str);
    assert_eq!(TypeALU::cast_to_type(Type::F64, &Type::Str, Span::default()).unwrap(), Type::Str);
    assert_eq!(TypeALU::cast_to_type(Type::I64, &Type::F64, Span::default()).unwrap(), Type::F64);
    assert_eq!(TypeALU::cast_to_type(Type::F64, &Type::I64, Span::default()).unwrap(), Type::I64);
    assert_eq!(TypeALU::cast_to_type(Type::I64, &Type::Bool, Span::default()).unwrap(), Type::Bool);
    assert_eq!(TypeALU::cast_to_type(Type::F64, &Type::Bool, Span::default()).unwrap(), Type::Bool);
    assert_eq!(TypeALU::cast_to_type(Type::Str, &Type::I64, Span::default()).unwrap(), Type::I64);
    assert_eq!(TypeALU::cast_to_type(Type::Str, &Type::F64, Span::default()).unwrap(), Type::F64);
    assert_eq!(TypeALU::cast_to_type(Type::Str, &Type::Bool, Span::default()).unwrap(), Type::Bool);
    assert_eq!(TypeALU::cast_to_type(Type::Bool, &Type::Str, Span::default()).unwrap(), Type::Str);
    assert_eq!(TypeALU::cast_to_type(Type::Bool, &Type::I64, Span::default()).unwrap(), Type::I64);
    assert_eq!(TypeALU::cast_to_type(Type::Bool, &Type::F64, Span::default()).unwrap(), Type::F64);
}

#[test]
fn cast_to_type_invalid() {
    assert_eq!(
        TypeALU::cast_to_type(Type::Vector(Box::new(Type::I64)), &Type::I64, Span::default())
            .err()
            .unwrap()
            .message(),
        "Cannot cast 'i64[]' to 'i64'."
    );
    assert_eq!(
        TypeALU::cast_to_type(Type::Void, &Type::I64, Span::default()).err().unwrap().message(),
        "Cannot cast 'void' to 'i64'."
    );
}

#[test]
fn cast_vector_to_supported_types_is_invalid() {
    assert_eq!(
        TypeALU::cast_to_type(Type::Vector(Box::new(Type::I64)), &Type::Str, Span::default())
            .err()
            .unwrap()
            .message(),
        "Cannot cast 'i64[]' to 'str'."
    );

    assert_eq!(
        TypeALU::cast_to_type(Type::Vector(Box::new(Type::I64)), &Type::Bool, Span::default())
            .err()
            .unwrap()
            .message(),
        "Cannot cast 'i64[]' to 'bool'."
    );
}

#[test]
fn cast_integer_to_integer_all_sizes() {
    let ints = [Type::I8, Type::I16, Type::I32, Type::I64, Type::U8, Type::U16, Type::U32, Type::U64];
    for from in &ints {
        for to in &ints {
            assert_eq!(TypeALU::cast_to_type(from.clone(), to, Span::default()).unwrap(), to.clone());
        }
    }
}

#[test]
fn cast_integer_to_str_f64_bool() {
    let ints = [Type::I8, Type::I16, Type::I32, Type::I64, Type::U8, Type::U16, Type::U32, Type::U64];
    for t in ints {
        assert_eq!(TypeALU::cast_to_type(t.clone(), &Type::Str, Span::default()).unwrap(), Type::Str);
        assert_eq!(TypeALU::cast_to_type(t.clone(), &Type::F64, Span::default()).unwrap(), Type::F64);
        assert_eq!(TypeALU::cast_to_type(t.clone(), &Type::Bool, Span::default()).unwrap(), Type::Bool);
    }
}

#[test]
fn cast_f64_to_all_integers() {
    let ints = [Type::I8, Type::I16, Type::I32, Type::I64, Type::U8, Type::U16, Type::U32, Type::U64];
    for t in ints {
        assert_eq!(TypeALU::cast_to_type(Type::F64, &t, Span::default()).unwrap(), t);
    }
}

#[test]
fn cast_str_to_all_integers() {
    let ints = [Type::I8, Type::I16, Type::I32, Type::I64, Type::U8, Type::U16, Type::U32, Type::U64];
    for t in ints {
        assert_eq!(TypeALU::cast_to_type(Type::Str, &t, Span::default()).unwrap(), t);
    }
}

#[test]
fn cast_bool_to_all_integers() {
    let ints = [Type::I8, Type::I16, Type::I32, Type::I64, Type::U8, Type::U16, Type::U32, Type::U64];
    for t in ints {
        assert_eq!(TypeALU::cast_to_type(Type::Bool, &t, Span::default()).unwrap(), t);
    }
}

#[test]
fn cast_char_conversions() {
    assert_eq!(TypeALU::cast_to_type(Type::Char, &Type::Str, Span::default()).unwrap(), Type::Str);
    assert_eq!(TypeALU::cast_to_type(Type::Char, &Type::U8, Span::default()).unwrap(), Type::U8);
    assert_eq!(TypeALU::cast_to_type(Type::U8, &Type::Char, Span::default()).unwrap(), Type::Char);
}

#[test]
fn cast_same_type_identity() {
    let types = [Type::I64, Type::F64, Type::Bool, Type::Str, Type::Char, Type::U8, Type::Void];
    for t in types {
        assert_eq!(TypeALU::cast_to_type(t.clone(), &t, Span::default()).unwrap(), t);
    }
}

#[test]
fn cast_unsupported_combinations_fail() {
    let bad = [
        (Type::Char, Type::I64),
        (Type::Char, Type::Bool),
        (Type::Char, Type::F64),
        (Type::I64, Type::Char),
        (Type::Str, Type::Char),
        (Type::Bool, Type::Char),
        (Type::F64, Type::Char),
        (Type::Vector(Box::new(Type::I64)), Type::F64),
        (Type::Void, Type::Str),
        (Type::I64, Type::Void),
    ];
    for (from, to) in bad {
        let r = TypeALU::cast_to_type(from.clone(), &to, Span::default());
        assert!(r.is_err(), "expected error for {} -> {}", from, to);
        let msg = r.err().unwrap().message();
        assert!(msg.starts_with("Cannot cast"), "msg: {}", msg);
    }
}
