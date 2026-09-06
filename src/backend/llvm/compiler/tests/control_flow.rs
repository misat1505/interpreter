use inkwell::context::Context;

use crate::{
    backend::llvm::compiler::{
        tests::{empty_program, node, span, with_main},
        ControlFrame,
    },
    frontend::ast::{Block, Expression, Literal, SwitchCase, SwitchExpression},
};

#[test]
fn branch_if_no_terminator_inserts_branch() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let function = compiler.main_fn.unwrap();
    let target = context.append_basic_block(function, "target");

    let entry = function.get_first_basic_block().unwrap();
    assert!(entry.get_terminator().is_none());

    compiler.branch_if_no_terminator(target, span()).unwrap();
    assert!(entry.get_terminator().is_some());
}

#[test]
fn branch_if_no_terminator_skips_when_already_terminated() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let function = compiler.main_fn.unwrap();
    let target = context.append_basic_block(function, "target");

    // terminate entry first
    let zero = compiler.i32_type().const_int(0, false);
    compiler.builder.build_return(Some(&zero)).unwrap();

    // should be a no-op
    assert!(compiler.branch_if_no_terminator(target, span()).is_ok());
}

#[test]
fn find_break_target_from_loop() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let function = compiler.main_fn.unwrap();
    let cont = context.append_basic_block(function, "cont");
    let brk = context.append_basic_block(function, "brk");

    compiler.control_stack.push(ControlFrame::Loop {
        continue_block: cont,
        break_block: brk,
        scope_depth: 1,
    });

    let found = compiler.find_break_target(span()).unwrap();
    assert_eq!(found, (brk, 1));
}

#[test]
fn find_break_target_from_switch() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let function = compiler.main_fn.unwrap();
    let after = context.append_basic_block(function, "after");

    compiler.control_stack.push(ControlFrame::Switch {
        break_block: after,
        scope_depth: 1,
    });

    let found = compiler.find_break_target(span()).unwrap();
    assert_eq!(found, (after, 1));
}

#[test]
fn find_break_target_prefers_innermost() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let function = compiler.main_fn.unwrap();
    let outer_brk = context.append_basic_block(function, "outer_brk");
    let inner_brk = context.append_basic_block(function, "inner_brk");
    let cont = context.append_basic_block(function, "cont");

    compiler.control_stack.push(ControlFrame::Loop {
        continue_block: cont,
        break_block: outer_brk,
        scope_depth: 1,
    });
    compiler.control_stack.push(ControlFrame::Switch {
        break_block: inner_brk,
        scope_depth: 1,
    });

    let found = compiler.find_break_target(span()).unwrap();
    assert_eq!(found, (inner_brk, 1));
}

#[test]
fn find_break_target_outside_fails() {
    let context = Context::create();
    let program = empty_program();
    let compiler = with_main(&program, &context);

    let err = compiler.find_break_target(span()).unwrap_err();
    assert!(err.message().contains("break"));
}

#[test]
fn find_continue_target_from_loop() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let function = compiler.main_fn.unwrap();
    let cont = context.append_basic_block(function, "cont");
    let brk = context.append_basic_block(function, "brk");

    compiler.control_stack.push(ControlFrame::Loop {
        continue_block: cont,
        break_block: brk,
        scope_depth: 1,
    });

    let found = compiler.find_continue_target(span()).unwrap();
    assert_eq!(found, (cont, 1));
}

#[test]
fn find_continue_target_skips_switch() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let function = compiler.main_fn.unwrap();
    let cont = context.append_basic_block(function, "cont");
    let brk = context.append_basic_block(function, "brk");
    let after = context.append_basic_block(function, "after");

    compiler.control_stack.push(ControlFrame::Loop {
        continue_block: cont,
        break_block: brk,
        scope_depth: 1,
    });
    compiler.control_stack.push(ControlFrame::Switch {
        break_block: after,
        scope_depth: 1,
    });

    // switch is ignored; loop's continue is found
    let found = compiler.find_continue_target(span()).unwrap();
    assert_eq!(found, (cont, 1));
}

#[test]
fn find_continue_target_outside_loop_fails() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    // only a switch on the stack
    let function = compiler.main_fn.unwrap();
    let after = context.append_basic_block(function, "after");
    compiler.control_stack.push(ControlFrame::Switch {
        break_block: after,
        scope_depth: 1,
    });

    let err = compiler.find_continue_target(span()).unwrap_err();
    assert!(err.message().contains("continue"));
}

#[test]
fn find_continue_target_empty_stack_fails() {
    let context = Context::create();
    let program = empty_program();
    let compiler = with_main(&program, &context);

    let err = compiler.find_continue_target(span()).unwrap_err();
    assert!(err.message().contains("continue"));
}

#[test]
fn compile_switch_single_true_case() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let cases = vec![node(SwitchCase {
        condition: node(Expression::Literal(Literal::True)),
        block: node(Block(vec![])),
    })];
    let expressions: Vec<_> = vec![];

    assert!(compiler.compile_switch(&expressions, &cases).is_ok());
    // control stack should be restored
    assert!(compiler.control_stack.is_empty());
}

#[test]
fn compile_switch_multiple_cases() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let cases = vec![
        node(SwitchCase {
            condition: node(Expression::Literal(Literal::False)),
            block: node(Block(vec![])),
        }),
        node(SwitchCase {
            condition: node(Expression::Literal(Literal::True)),
            block: node(Block(vec![])),
        }),
    ];
    let expressions: Vec<_> = vec![];

    assert!(compiler.compile_switch(&expressions, &cases).is_ok());
    assert!(compiler.control_stack.is_empty());
}

#[test]
fn compile_switch_with_alias_binding() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let expressions = vec![node(SwitchExpression {
        expression: node(Expression::Literal(Literal::I64(42))),
        alias: Some(node(String::from("x"))),
    })];
    let cases = vec![node(SwitchCase {
        condition: node(Expression::Literal(Literal::True)),
        block: node(Block(vec![])),
    })];

    assert!(compiler.compile_switch(&expressions, &cases).is_ok());
    // alias should be removed after switch (variables restored)
    assert!(compiler.get_variable("x").is_err());
}

#[test]
fn compile_switch_non_bool_condition_fails() {
    let context = Context::create();
    let program = empty_program();
    let mut compiler = with_main(&program, &context);

    let cases = vec![node(SwitchCase {
        condition: node(Expression::Literal(Literal::I64(1))),
        block: node(Block(vec![])),
    })];
    let expressions: Vec<_> = vec![];

    // into_int_value requires Bool
    assert!(compiler.compile_switch(&expressions, &cases).is_err());
}
