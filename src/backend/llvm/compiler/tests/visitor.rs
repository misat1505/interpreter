use inkwell::context::Context;

use crate::{
    backend::llvm::{
        compiler::{
            tests::{empty_program, node, span},
            Compiler,
        },
        OverflowPolicy,
    },
    common::{types::Type, visitor::Visitor},
    frontend::ast::{
        Argument, Block, Expression, Literal, Parameter, PassedBy, Program, Statement, SwitchCase, SwitchExpression, VariableDeclarationKind,
    },
};

fn with_main<'a, 'ctx>(program: &'a Program, context: &'ctx Context) -> Compiler<'a, 'ctx> {
    let mut c = Compiler::new(program, context, OverflowPolicy::Ignore);
    c.declare_main_function();
    c
}

#[test]
fn visit_empty_program() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    assert!(compiler.visit_program(&program).is_ok());
}

#[test]
fn visit_program_with_statements() {
    let context = Context::create();
    let mut program = empty_program();
    program.statements = vec![
        node(Statement::Declaration {
            identifier: node(String::from("x")),
            kind: VariableDeclarationKind::TYPE {
                var_type: node(Type::I64),
                value: Some(node(Expression::Literal(Literal::I64(1)))),
            },
        }),
        node(Statement::Declaration {
            identifier: node(String::from("b")),
            kind: VariableDeclarationKind::TYPE {
                var_type: node(Type::Bool),
                value: Some(node(Expression::Literal(Literal::True))),
            },
        }),
    ];
    let mut compiler = with_main(&program, &context);

    assert!(compiler.visit_program(&program).is_ok());
}

#[test]
fn visit_all_literals() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    compiler.visit_literal(&Literal::I64(42)).unwrap();
    assert_eq!(compiler.read_last_value().unwrap().to_type(), Type::I64);

    compiler.visit_literal(&Literal::F64(2.5)).unwrap();
    assert_eq!(compiler.read_last_value().unwrap().to_type(), Type::F64);

    compiler.visit_literal(&Literal::Char('z')).unwrap();
    assert_eq!(compiler.read_last_value().unwrap().to_type(), Type::Char);

    compiler.visit_literal(&Literal::True).unwrap();
    assert_eq!(compiler.read_last_value().unwrap().to_type(), Type::Bool);

    compiler.visit_literal(&Literal::False).unwrap();
    assert_eq!(compiler.read_last_value().unwrap().to_type(), Type::Bool);

    let binding = Literal::String(String::from("hello"));
    compiler.visit_literal(&binding).unwrap();
    assert_eq!(compiler.read_last_value().unwrap().to_type(), Type::Str);
}

#[test]
fn visit_variable_loads_value() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    // declare via statement so alloca + store happen
    let binding = node(Statement::Declaration {
        identifier: node(String::from("n")),
        kind: VariableDeclarationKind::TYPE {
            var_type: node(Type::I64),
            value: Some(node(Expression::Literal(Literal::I64(7)))),
        },
    });

    compiler.visit_statement(&binding).unwrap();

    let binding = String::from("n");
    assert!(compiler.visit_variable(&binding, span()).is_ok());
    assert_eq!(compiler.read_last_value().unwrap().to_type(), Type::I64);
}

#[test]
fn visit_variable_undeclared_fails() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    assert!(compiler.visit_variable(&String::from("missing"), span()).is_err());
}

#[test]
fn visit_empty_block() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let block = node(Block(vec![]));
    assert!(compiler.visit_block(&block).is_ok());
}

#[test]
fn visit_block_with_statements() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let block = node(Block(vec![
        node(Statement::Declaration {
            identifier: node(String::from("a")),
            kind: VariableDeclarationKind::TYPE {
                var_type: node(Type::I64),
                value: Some(node(Expression::Literal(Literal::I64(1)))),
            },
        }),
        node(Statement::Assignment {
            identifier: node(String::from("a")),
            value: node(Expression::Literal(Literal::I64(2))),
            accessors: vec![],
        }),
    ]));
    assert!(compiler.visit_block(&block).is_ok());
}

// ─── visit_expression / visit_statement (dispatch) ──────────────────────────

#[test]
fn visit_expression_dispatches_to_compile() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let expr = node(Expression::Literal(Literal::I64(10)));
    assert!(compiler.visit_expression(&expr).is_ok());
    assert_eq!(compiler.read_last_value().unwrap().to_type(), Type::I64);
}

#[test]
fn visit_statement_dispatches_to_compile() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let stmt = node(Statement::Declaration {
        identifier: node(String::from("ok")),
        kind: VariableDeclarationKind::TYPE {
            var_type: node(Type::Bool),
            value: Some(node(Expression::Literal(Literal::False))),
        },
    });
    assert!(compiler.visit_statement(&stmt).is_ok());
}

#[test]
fn visit_parameter_argument_type_are_noop() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let param = node(Parameter {
        identifier: node(String::from("p")),
        parameter_type: node(Type::I64),
        passed_by: PassedBy::Value,
    });
    assert!(compiler.visit_parameter(&param).is_ok());

    let arg = node(Argument {
        value: node(Expression::Literal(Literal::I64(1))),
        passed_by: PassedBy::Value,
    });
    assert!(compiler.visit_argument(&arg).is_ok());

    let ty = node(Type::F64);
    assert!(compiler.visit_type(&ty).is_ok());
}

#[test]
fn visit_switch_expression_and_case_are_noop() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let se = node(SwitchExpression {
        expression: node(Expression::Literal(Literal::I64(1))),
        alias: None,
    });
    assert!(compiler.visit_switch_expression(&se).is_ok());

    let sc = node(SwitchCase {
        condition: node(Expression::Literal(Literal::True)),
        block: node(Block(vec![])),
    });
    assert!(compiler.visit_switch_case(&sc).is_ok());
}

#[test]
fn visit_vector_literal_not_supported() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let elements: Vec<Box<_>> = vec![];
    let err = compiler.visit_vector_literal(&elements).unwrap_err();
    assert!(err.message().contains("not yet supported"));
}
