use std::assert_eq;

use crate::backend::interpreter::alu::ALU;
use crate::common::errors::IError;
use crate::common::span::Span;
use crate::{backend::interpreter::alu::value::Value, common::types::Type};

#[test]
fn cast_to_type() {
    let data = [
        (Value::I64(1), Type::Str),
        (Value::F64(1.2), Type::Str),
        (Value::I64(1), Type::F64),
        (Value::F64(1.2), Type::I64),
        (Value::I64(1), Type::Bool),
        (Value::I64(0), Type::Bool),
        (Value::F64(1.2), Type::Bool),
        (Value::F64(0.0), Type::Bool),
        (Value::String(String::from("1")), Type::I64),
        (Value::String(String::from("1.2")), Type::F64),
        (Value::String(String::from("some string")), Type::Bool),
        (Value::String(String::from("")), Type::Bool),
    ];

    let expected = [
        Value::String(String::from("1")),
        Value::String(String::from("1.2")),
        Value::F64(1.0),
        Value::I64(1),
        Value::Bool(true),
        Value::Bool(false),
        Value::Bool(true),
        Value::Bool(false),
        Value::I64(1),
        Value::F64(1.2),
        Value::Bool(true),
        Value::Bool(false),
    ];

    for idx in 0..data.len() {
        let (init, to_type) = &data[idx];
        let exp = &expected[idx];
        assert_eq!(ALU::cast_to_type(init.clone(), to_type, Span::default()).unwrap(), *exp);
    }
}

#[test]
fn cast_to_type_fail() {
    let data = [
        (Value::String(String::from("abc")), Type::I64),
        (Value::String(String::from("abc")), Type::F64),
    ];

    for (val, to_type) in data {
        assert_eq!(
            ALU::cast_to_type(val, &to_type, Span::default()).err().unwrap().message(),
            format!("Cannot cast String 'abc' to '{}'.", to_type)
        );
    }
}

#[test]
fn cast_bool_to_other_types() {
    assert_eq!(
        ALU::cast_to_type(Value::Bool(true), &Type::Str, Span::default()).unwrap(),
        Value::String(String::from("true"))
    );
    assert_eq!(
        ALU::cast_to_type(Value::Bool(false), &Type::Str, Span::default()).unwrap(),
        Value::String(String::from("false"))
    );
    assert_eq!(ALU::cast_to_type(Value::Bool(true), &Type::I64, Span::default()).unwrap(), Value::I64(1));
    assert_eq!(ALU::cast_to_type(Value::Bool(false), &Type::I64, Span::default()).unwrap(), Value::I64(0));
    assert_eq!(
        ALU::cast_to_type(Value::Bool(true), &Type::F64, Span::default()).unwrap(),
        Value::F64(1.0)
    );
    assert_eq!(
        ALU::cast_to_type(Value::Bool(false), &Type::F64, Span::default()).unwrap(),
        Value::F64(0.0)
    );
}

#[test]
fn cast_to_type_unsupported_combination_fails() {
    let result = ALU::cast_to_type(Value::I64(1), &Type::Char, Span::default());
    assert!(result.is_err());
    assert_eq!(result.err().unwrap().message(), String::from("Cannot cast '1' to 'char'."));
}

#[test]
fn cast_integer_to_string_all_sizes() {
    assert_eq!(
        ALU::cast_to_type(Value::I8(42), &Type::Str, Span::default()).unwrap(),
        Value::String("42".to_string())
    );
    assert_eq!(
        ALU::cast_to_type(Value::I16(-1000), &Type::Str, Span::default()).unwrap(),
        Value::String("-1000".to_string())
    );
    assert_eq!(
        ALU::cast_to_type(Value::I32(123456), &Type::Str, Span::default()).unwrap(),
        Value::String("123456".to_string())
    );
    assert_eq!(
        ALU::cast_to_type(Value::U8(255), &Type::Str, Span::default()).unwrap(),
        Value::String("255".to_string())
    );
    assert_eq!(
        ALU::cast_to_type(Value::U16(65535), &Type::Str, Span::default()).unwrap(),
        Value::String("65535".to_string())
    );
    assert_eq!(
        ALU::cast_to_type(Value::U32(4294967295), &Type::Str, Span::default()).unwrap(),
        Value::String("4294967295".to_string())
    );
    assert_eq!(
        ALU::cast_to_type(Value::U64(18446744073709551615), &Type::Str, Span::default()).unwrap(),
        Value::String("18446744073709551615".to_string())
    );
}

#[test]
fn cast_integer_to_f64() {
    assert_eq!(ALU::cast_to_type(Value::I8(-5), &Type::F64, Span::default()).unwrap(), Value::F64(-5.0));
    assert_eq!(
        ALU::cast_to_type(Value::U64(100), &Type::F64, Span::default()).unwrap(),
        Value::F64(100.0)
    );
}

#[test]
fn cast_f64_to_integer_success() {
    assert_eq!(ALU::cast_to_type(Value::F64(42.9), &Type::I8, Span::default()).unwrap(), Value::I8(42));
    assert_eq!(
        ALU::cast_to_type(Value::F64(-100.1), &Type::I16, Span::default()).unwrap(),
        Value::I16(-100)
    );
    assert_eq!(ALU::cast_to_type(Value::F64(0.0), &Type::U8, Span::default()).unwrap(), Value::U8(0));
    assert_eq!(ALU::cast_to_type(Value::F64(155.9), &Type::U8, Span::default()).unwrap(), Value::U8(155));
    assert_eq!(
        ALU::cast_to_type(Value::F64(1e10), &Type::I64, Span::default()).unwrap(),
        Value::I64(10_000_000_000)
    );
}

#[test]
fn cast_f64_to_integer_out_of_range_fails() {
    // i8 overflow
    let r = ALU::cast_to_type(Value::F64(128.0), &Type::I8, Span::default());
    assert!(r.is_err());
    assert_eq!(r.err().unwrap().message(), "Cannot cast '128' to 'i8'.");

    // i8 underflow
    let r = ALU::cast_to_type(Value::F64(-129.0), &Type::I8, Span::default());
    assert!(r.is_err());

    // u8 negative
    let r = ALU::cast_to_type(Value::F64(-1.0), &Type::U8, Span::default());
    assert!(r.is_err());
    assert_eq!(r.err().unwrap().message(), "Cannot cast '-1' to 'u8'.");

    // u8 overflow
    let r = ALU::cast_to_type(Value::F64(256.0), &Type::U8, Span::default());
    assert!(r.is_err());

    // non-finite
    let r = ALU::cast_to_type(Value::F64(f64::INFINITY), &Type::I32, Span::default());
    assert!(r.is_err());
    let r = ALU::cast_to_type(Value::F64(f64::NAN), &Type::U64, Span::default());
    assert!(r.is_err());
}

#[test]
fn cast_integer_to_bool() {
    assert_eq!(
        ALU::cast_to_type(Value::I8(-1), &Type::Bool, Span::default()).unwrap(),
        Value::Bool(false) // only > 0 is true
    );
    assert_eq!(ALU::cast_to_type(Value::I8(0), &Type::Bool, Span::default()).unwrap(), Value::Bool(false));
    assert_eq!(ALU::cast_to_type(Value::I8(1), &Type::Bool, Span::default()).unwrap(), Value::Bool(true));
    assert_eq!(
        ALU::cast_to_type(Value::U64(0), &Type::Bool, Span::default()).unwrap(),
        Value::Bool(false)
    );
    assert_eq!(
        ALU::cast_to_type(Value::U64(999), &Type::Bool, Span::default()).unwrap(),
        Value::Bool(true)
    );
}

#[test]
fn cast_string_to_integer_all_sizes() {
    assert_eq!(
        ALU::cast_to_type(Value::String("-128".into()), &Type::I8, Span::default()).unwrap(),
        Value::I8(-128)
    );
    assert_eq!(
        ALU::cast_to_type(Value::String("127".into()), &Type::I8, Span::default()).unwrap(),
        Value::I8(127)
    );
    assert_eq!(
        ALU::cast_to_type(Value::String("255".into()), &Type::U8, Span::default()).unwrap(),
        Value::U8(255)
    );
    assert_eq!(
        ALU::cast_to_type(Value::String("0".into()), &Type::U64, Span::default()).unwrap(),
        Value::U64(0)
    );
}

#[test]
fn cast_string_to_integer_invalid_fails() {
    let cases = [
        ("abc", Type::I8),
        ("128", Type::I8),  // overflow
        ("-1", Type::U8),   // negative
        ("256", Type::U8),  // overflow
        ("1.5", Type::I32), // not integer
        ("", Type::I64),
    ];
    for (s, ty) in cases {
        let r = ALU::cast_to_type(Value::String(s.into()), &ty, Span::default());
        assert!(r.is_err(), "expected error for '{}' -> {}", s, ty);
        let msg = r.err().unwrap().message();
        assert!(msg.contains("Cannot cast String"), "msg was: {}", msg);
    }
}

#[test]
fn cast_char_u8_roundtrip() {
    assert_eq!(ALU::cast_to_type(Value::Char('A'), &Type::U8, Span::default()).unwrap(), Value::U8(65));
    assert_eq!(ALU::cast_to_type(Value::U8(65), &Type::Char, Span::default()).unwrap(), Value::Char('A'));
    assert_eq!(ALU::cast_to_type(Value::Char('\0'), &Type::U8, Span::default()).unwrap(), Value::U8(0));
    assert_eq!(
        ALU::cast_to_type(Value::U8(255), &Type::Char, Span::default()).unwrap(),
        Value::Char(255 as char)
    );
}

#[test]
fn cast_char_to_string() {
    assert_eq!(
        ALU::cast_to_type(Value::Char('€'), &Type::Str, Span::default()).unwrap(),
        Value::String("€".to_string())
    );
    assert_eq!(
        ALU::cast_to_type(Value::Char('x'), &Type::Str, Span::default()).unwrap(),
        Value::String("x".to_string())
    );
}

#[test]
fn cast_same_type_identity() {
    let values = [
        Value::I32(42),
        Value::F64(3.14),
        Value::Bool(true),
        Value::String("hello".into()),
        Value::Char('z'),
        Value::U64(0),
    ];
    for v in values {
        let ty = v.to_type();
        assert_eq!(ALU::cast_to_type(v.clone(), &ty, Span::default()).unwrap(), v);
    }
}

#[test]
fn cast_integer_to_integer_success() {
    // widening
    assert_eq!(ALU::cast_to_type(Value::I8(-5), &Type::I64, Span::default()).unwrap(), Value::I64(-5));
    assert_eq!(ALU::cast_to_type(Value::U8(200), &Type::U64, Span::default()).unwrap(), Value::U64(200));
    // signed -> unsigned (in range)
    assert_eq!(ALU::cast_to_type(Value::I16(100), &Type::U32, Span::default()).unwrap(), Value::U32(100));
    // unsigned -> signed (in range)
    assert_eq!(ALU::cast_to_type(Value::U16(100), &Type::I32, Span::default()).unwrap(), Value::I32(100));
}

#[test]
fn cast_integer_to_integer_out_of_range_fails() {
    // i8 overflow
    let r = ALU::cast_to_type(Value::I16(128), &Type::I8, Span::default());
    assert!(r.is_err());
    assert_eq!(r.err().unwrap().message(), "Cannot cast '128' to 'i8'.");

    // negative to unsigned
    let r = ALU::cast_to_type(Value::I32(-1), &Type::U8, Span::default());
    assert!(r.is_err());
    assert_eq!(r.err().unwrap().message(), "Cannot cast '-1' to 'u8'.");

    // u16 overflow into i8
    let r = ALU::cast_to_type(Value::U16(300), &Type::I8, Span::default());
    assert!(r.is_err());
}

#[test]
fn cast_bool_to_all_integer_types() {
    for &b in &[true, false] {
        let expected = if b { 1 } else { 0 };
        assert_eq!(
            ALU::cast_to_type(Value::Bool(b), &Type::I8, Span::default()).unwrap(),
            Value::I8(expected as i8)
        );
        assert_eq!(
            ALU::cast_to_type(Value::Bool(b), &Type::I16, Span::default()).unwrap(),
            Value::I16(expected as i16)
        );
        assert_eq!(
            ALU::cast_to_type(Value::Bool(b), &Type::I32, Span::default()).unwrap(),
            Value::I32(expected as i32)
        );
        assert_eq!(
            ALU::cast_to_type(Value::Bool(b), &Type::I64, Span::default()).unwrap(),
            Value::I64(expected as i64)
        );
        assert_eq!(
            ALU::cast_to_type(Value::Bool(b), &Type::U8, Span::default()).unwrap(),
            Value::U8(expected as u8)
        );
        assert_eq!(
            ALU::cast_to_type(Value::Bool(b), &Type::U16, Span::default()).unwrap(),
            Value::U16(expected as u16)
        );
        assert_eq!(
            ALU::cast_to_type(Value::Bool(b), &Type::U32, Span::default()).unwrap(),
            Value::U32(expected as u32)
        );
        assert_eq!(
            ALU::cast_to_type(Value::Bool(b), &Type::U64, Span::default()).unwrap(),
            Value::U64(expected as u64)
        );
    }
}

#[test]
fn cast_unsupported_combinations() {
    let unsupported = [
        (Value::String("x".into()), Type::Char),
        (Value::F64(1.0), Type::Char),
        (Value::Bool(true), Type::Char),
        (Value::Char('a'), Type::I32),
        (Value::Char('a'), Type::Bool),
        (Value::I64(1), Type::Char),
    ];
    for (val, ty) in unsupported {
        let r = ALU::cast_to_type(val, &ty, Span::default());
        assert!(r.is_err());
        let msg = r.err().unwrap().message();
        assert!(msg.starts_with("Cannot cast"), "msg: {}", msg);
    }
}
