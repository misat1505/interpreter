use inkwell::context::Context;

use crate::{
    backend::llvm::{
        compiler::{
            tests::{empty_program, node, span},
            Compiler,
        },
        OverflowPolicy,
    },
    common::types::Type,
    frontend::ast::{Accessor, Expression, Literal},
};

fn with_main<'a, 'ctx>(program: &'a crate::frontend::ast::Program, context: &'ctx Context) -> Compiler<'a, 'ctx> {
    let mut c = Compiler::new(program, context, OverflowPolicy::Ignore);
    c.declare_main_function();
    c
}

#[test]
fn build_empty_vector_succeeds() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let ptr = compiler.build_empty_vector(&Type::I64, span()).unwrap();
    // just a non-null-typed pointer value in IR (const_null would still be a PointerValue)
    let _ = ptr;
}

#[test]
fn build_empty_vector_various_inner_types() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    for ty in [Type::I64, Type::F64, Type::Bool, Type::Str, Type::Char] {
        assert!(compiler.build_empty_vector(&ty, span()).is_ok());
    }
}

#[test]
fn build_vector_from_i64_elements() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let elements = vec![
        Box::new(node(Expression::Literal(Literal::I64(1)))),
        Box::new(node(Expression::Literal(Literal::I64(2)))),
        Box::new(node(Expression::Literal(Literal::I64(3)))),
    ];
    assert!(compiler.build_vector_from_elements(&Type::I64, &elements, None, span()).is_ok());
}

#[test]
fn build_vector_type_mismatch_fails() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let elements = vec![
        Box::new(node(Expression::Literal(Literal::I64(1)))),
        Box::new(node(Expression::Literal(Literal::True))), // wrong type
    ];
    let err = compiler.build_vector_from_elements(&Type::I64, &elements, None, span()).unwrap_err();
    assert!(err.message().contains("mismatch"));
}

#[test]
fn build_vector_with_precomputed_first() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let first = crate::backend::llvm::llvm_alu::llvm_value::LlvmValue::I64(context.i64_type().const_int(10, true));
    let elements = vec![
        Box::new(node(Expression::Literal(Literal::I64(10)))),
        Box::new(node(Expression::Literal(Literal::I64(20)))),
    ];
    assert!(compiler.build_vector_from_elements(&Type::I64, &elements, Some(first), span()).is_ok());
}

#[test]
fn build_vector_expression_infers_type() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let elements = vec![
        Box::new(node(Expression::Literal(Literal::I64(1)))),
        Box::new(node(Expression::Literal(Literal::I64(2)))),
    ];
    let (_ptr, inner) = compiler.build_vector_expression(&elements, span()).unwrap();
    assert_eq!(inner, Type::I64);
}

#[test]
fn build_vector_expression_empty_fails() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let elements: Vec<Box<_>> = vec![];
    let err = compiler.build_vector_expression(&elements, span()).unwrap_err();
    assert!(err.message().contains("empty vector"));
}

#[test]
fn build_vector_expression_f64() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let elements = vec![
        Box::new(node(Expression::Literal(Literal::F64(1.0)))),
        Box::new(node(Expression::Literal(Literal::F64(2.5)))),
    ];
    let (_ptr, inner) = compiler.build_vector_expression(&elements, span()).unwrap();
    assert_eq!(inner, Type::F64);
}

#[test]
fn evaluate_scalar_vector_element() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let elem = node(Expression::Literal(Literal::I64(5)));
    let v = compiler.evaluate_vector_element(&Type::I64, &elem).unwrap();
    assert_eq!(v.to_type(), Type::I64);
}

#[test]
fn evaluate_nested_empty_vector_element() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let nested_ty = Type::Vector(Box::new(Type::I64));
    let elem = node(Expression::Vector(vec![]));
    let v = compiler.evaluate_vector_element(&nested_ty, &elem).unwrap();
    assert_eq!(v.to_type(), Type::Vector(Box::new(Type::I64)));
}

#[test]
fn evaluate_nested_nonempty_vector_element() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let nested_ty = Type::Vector(Box::new(Type::I64));
    let elem = node(Expression::Vector(vec![
        Box::new(node(Expression::Literal(Literal::I64(1)))),
        Box::new(node(Expression::Literal(Literal::I64(2)))),
    ]));
    let v = compiler.evaluate_vector_element(&nested_ty, &elem).unwrap();
    assert_eq!(v.to_type(), Type::Vector(Box::new(Type::I64)));
}

// TODO: build_shallow_copy_vector got deleted
// #[test]
// fn shallow_copy_empty_vector() {
//     let context = Context::create();
//     let program = empty_program();
//     let mut compiler = with_main(&program, &context);

//     let src = compiler.build_empty_vector(&Type::I64, span()).unwrap();
//     assert!(compiler.build_shallow_copy_vector(src, &Type::I64, span()).is_ok());
// }

// TODO: build_shallow_copy_vector got deleted
// #[test]
// fn shallow_copy_nonempty_vector() {
//     let context = Context::create();
//     let program = empty_program();
//     let mut compiler = with_main(&program, &context);

//     let elements = vec![
//         Box::new(node(Expression::Literal(Literal::I64(1)))),
//         Box::new(node(Expression::Literal(Literal::I64(2)))),
//     ];
//     let src = compiler.build_vector_from_elements(&Type::I64, &elements, None, span()).unwrap();
//     assert!(compiler.build_shallow_copy_vector(src, &Type::I64, span()).is_ok());
// }

#[test]
fn resolve_index_empty_indices_fails() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let vec_ptr = compiler.build_empty_vector(&Type::I64, span()).unwrap();
    let err = compiler
        .resolve_indexed_element(vec_ptr, &Type::Vector(Box::new(Type::I64)), &[], span())
        .unwrap_err();
    assert!(err.message().contains("at least one accessor"));
}

#[test]
fn resolve_index_into_vector() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let elements = vec![
        Box::new(node(Expression::Literal(Literal::I64(10)))),
        Box::new(node(Expression::Literal(Literal::I64(20)))),
    ];
    let vec_ptr = compiler.build_vector_from_elements(&Type::I64, &elements, None, span()).unwrap();

    let accessors = [node(Accessor::Index(node(Expression::Literal(Literal::I64(1)))))];
    let (elem_ptr, elem_ty) = compiler
        .resolve_indexed_element(vec_ptr, &Type::Vector(Box::new(Type::I64)), &accessors, span())
        .unwrap();
    assert_eq!(elem_ty, Type::I64);
    let _ = elem_ptr;
}

#[test]
fn resolve_index_into_non_vector_fails() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    // use a dummy pointer with type I64
    let dummy = context.ptr_type(inkwell::AddressSpace::default()).const_null();
    let accessors = [node(Accessor::Index(node(Expression::Literal(Literal::I64(0)))))];
    let err = compiler.resolve_indexed_element(dummy, &Type::I64, &accessors, span()).unwrap_err();
    assert!(err.message().contains("Cannot index"));
}

#[test]
fn build_default_value_scalars() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    for ty in [Type::I64, Type::F64, Type::Bool, Type::Str] {
        let llvm_ty = crate::backend::llvm::llvm_alu::llvm_value::LlvmValue::type_to_basic_type_enum(&ty, &context).unwrap();
        let ptr = compiler.builder.build_alloca(llvm_ty, "def").unwrap();
        assert!(compiler.build_default_value(ptr, &ty, span()).is_ok());
    }
}

#[test]
fn build_default_value_vector() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let ty = Type::Vector(Box::new(Type::I64));
    let llvm_ty = context.ptr_type(inkwell::AddressSpace::default());
    let ptr = compiler.builder.build_alloca(llvm_ty, "v").unwrap();
    assert!(compiler.build_default_value(ptr, &ty, span()).is_ok());
}

#[test]
fn build_default_value_unsupported_fails() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let llvm_ty = context.i8_type();
    let ptr = compiler.builder.build_alloca(llvm_ty, "c").unwrap();
    // Char is not handled in build_default_value
    let err = compiler.build_default_value(ptr, &Type::Char, span()).unwrap_err();
    assert!(err.message().contains("not yet supported"));
}
