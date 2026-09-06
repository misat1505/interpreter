use inkwell::context::Context;

use crate::{
    backend::llvm::{
        compiler::{
            tests::{empty_program, span},
            Compiler,
        },
        llvm_alu::llvm_value::LlvmValue,
        OverflowPolicy,
    },
    common::types::Type,
    frontend::ast::{Expression, Literal},
};

fn with_main<'a, 'ctx>(program: &'a crate::frontend::ast::Program, context: &'ctx Context) -> Compiler<'a, 'ctx> {
    let mut c = Compiler::new(program, context, OverflowPolicy::Ignore);
    c.declare_main_function();
    c
}

#[test]
fn format_signed_integers() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let cases = [
        (Type::I8, LlvmValue::I8(context.i8_type().const_int(7, true))),
        (Type::I16, LlvmValue::I16(context.i16_type().const_int(7, true))),
        (Type::I32, LlvmValue::I32(context.i32_type().const_int(7, true))),
        (Type::I64, LlvmValue::I64(context.i64_type().const_int(7, true))),
    ];
    for (ty, val) in cases {
        assert!(compiler.format_scalar_to_cstring(val, &ty, span()).is_ok(), "failed for {}", ty);
    }
}

#[test]
fn format_unsigned_integers() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let cases = [
        (Type::U8, LlvmValue::U8(context.i8_type().const_int(7, false))),
        (Type::U16, LlvmValue::U16(context.i16_type().const_int(7, false))),
        (Type::U32, LlvmValue::U32(context.i32_type().const_int(7, false))),
        (Type::U64, LlvmValue::U64(context.i64_type().const_int(7, false))),
    ];
    for (ty, val) in cases {
        assert!(compiler.format_scalar_to_cstring(val, &ty, span()).is_ok(), "failed for {}", ty);
    }
}

#[test]
fn format_f64() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let v = LlvmValue::F64(context.f64_type().const_float(3.14));
    assert!(compiler.format_scalar_to_cstring(v, &Type::F64, span()).is_ok());
}

#[test]
fn format_char() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let v = LlvmValue::Char(context.i8_type().const_int(b'A' as u64, false));
    assert!(compiler.format_scalar_to_cstring(v, &Type::Char, span()).is_ok());
}

#[test]
fn format_bool_true_and_false() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let t = LlvmValue::Bool(context.bool_type().const_int(1, false));
    let f = LlvmValue::Bool(context.bool_type().const_int(0, false));
    assert!(compiler.format_scalar_to_cstring(t, &Type::Bool, span()).is_ok());
    assert!(compiler.format_scalar_to_cstring(f, &Type::Bool, span()).is_ok());
}

#[test]
fn format_str() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    // use a global empty string as stand-in
    let s = compiler.builder.build_global_string_ptr("hello", "s").unwrap().as_pointer_value();
    let v = LlvmValue::Str(s);
    assert!(compiler.format_scalar_to_cstring(v, &Type::Str, span()).is_ok());
}

#[test]
fn format_unsupported_type_fails() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    // Vector is not a scalar for format_scalar
    let v = LlvmValue::I64(context.i64_type().const_zero());
    let err = compiler.format_scalar_to_cstring(v, &Type::Void, span()).unwrap_err();
    assert!(err.message().contains("not yet supported"));
}

#[test]
fn append_cstring_tracked_grows_buffer() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let ptr_ty = context.ptr_type(inkwell::AddressSpace::default());
    let i64_ty = context.i64_type();

    // initial buffer with "["
    let init = compiler.builder.build_global_string_ptr("[", "init").unwrap().as_pointer_value();
    // heap-copy so realloc is valid in IR
    let len = i64_ty.const_int(2, false);
    let heap = compiler
        .builder
        .build_call(compiler.libc.malloc_fn, &[len.into()], "init.heap")
        .unwrap()
        .try_as_basic_value()
        .basic()
        .unwrap()
        .into_pointer_value();
    compiler
        .builder
        .build_call(compiler.libc.strcpy_fn, &[heap.into(), init.into()], "init.copy")
        .unwrap();

    let buffer_ptr = compiler.builder.build_alloca(ptr_ty, "buf").unwrap();
    compiler.builder.build_store(buffer_ptr, heap).unwrap();

    let addition = compiler.builder.build_global_string_ptr("1", "add").unwrap().as_pointer_value();
    let current_len = i64_ty.const_int(1, false); // len of "["

    let new_len = compiler.append_cstring_tracked(buffer_ptr, current_len, addition, span()).unwrap();
    let _ = new_len;
}

#[test]
fn vector_to_string_empty() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let vec_ptr = compiler.build_empty_vector(&Type::I64, span()).unwrap();
    assert!(compiler.build_vector_to_string(vec_ptr, &Type::I64, span()).is_ok());
}

#[test]
fn vector_to_string_nonempty_i64() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let elements = vec![
        Box::new(crate::backend::llvm::compiler::tests::node(Expression::Literal(Literal::I64(1)))),
        Box::new(crate::backend::llvm::compiler::tests::node(Expression::Literal(Literal::I64(2)))),
    ];
    let vec_ptr = compiler.build_vector_from_elements(&Type::I64, &elements, None, span()).unwrap();
    assert!(compiler.build_vector_to_string(vec_ptr, &Type::I64, span()).is_ok());
}

#[test]
fn vector_to_string_bools() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let elements = vec![
        Box::new(crate::backend::llvm::compiler::tests::node(Expression::Literal(Literal::True))),
        Box::new(crate::backend::llvm::compiler::tests::node(Expression::Literal(Literal::False))),
    ];
    let vec_ptr = compiler.build_vector_from_elements(&Type::Bool, &elements, None, span()).unwrap();
    assert!(compiler.build_vector_to_string(vec_ptr, &Type::Bool, span()).is_ok());
}

#[test]
fn vector_to_string_nested() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    // outer: vector of vector of i64, with one inner [1, 2]
    let inner_ty = Type::Vector(Box::new(Type::I64));
    let inner_elements = vec![
        Box::new(crate::backend::llvm::compiler::tests::node(Expression::Literal(Literal::I64(1)))),
        Box::new(crate::backend::llvm::compiler::tests::node(Expression::Literal(Literal::I64(2)))),
    ];
    let outer_elements = vec![Box::new(crate::backend::llvm::compiler::tests::node(Expression::Vector(inner_elements)))];
    let vec_ptr = compiler.build_vector_from_elements(&inner_ty, &outer_elements, None, span()).unwrap();
    assert!(compiler.build_vector_to_string(vec_ptr, &inner_ty, span()).is_ok());
}
